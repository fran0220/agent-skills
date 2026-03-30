use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

mod client;
mod config;
mod frontend;
mod mcp;
mod output;
mod providers;
mod scoring;
mod search;
mod server;

use crate::config::{AppConfig, SEARCH_MODELS, SEARCH_MODES};
use crate::search::{SearchEngine, SearchMode};

#[derive(Parser)]
#[command(name = "ai-search", about = "AI-powered web search", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Search query (shorthand for `search` subcommand)
    #[arg(trailing_var_arg = true)]
    query: Vec<String>,

    /// Model to use
    #[arg(short, long, global = true)]
    model: Option<String>,

    /// Max sub-queries for query splitting
    #[arg(long, global = true)]
    split: Option<u32>,

    /// Search mode: fast, deep, or answer
    #[arg(long, global = true)]
    mode: Option<String>,

    /// Output full JSON
    #[arg(long, global = true)]
    json: bool,

    /// Human-readable output
    #[arg(long, global = true)]
    human: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Search the web
    Search {
        /// Search query
        query: Vec<String>,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Max sub-queries for query splitting
        #[arg(long)]
        split: Option<u32>,
        /// Search mode: fast, deep, or answer
        #[arg(long)]
        mode: Option<String>,
        /// Output full JSON with usage
        #[arg(long)]
        json: bool,
        /// Human-readable output
        #[arg(long)]
        human: bool,
        /// Read query from stdin JSON
        #[arg(long)]
        stdin: bool,
    },
    /// Start MCP/HTTP server
    Serve {
        /// Port for HTTP mode
        #[arg(short, long)]
        port: Option<u16>,
        /// Config file path
        #[arg(long)]
        config: Option<PathBuf>,
        /// Run in MCP stdio mode
        #[arg(long)]
        stdio: bool,
    },
    /// List available search models
    Models,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { run(cli).await })
}

async fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Some(Commands::Search {
            query,
            model,
            split,
            mode,
            json,
            human,
            stdin,
        }) => run_search(query, mode, model, split, json, human, stdin).await,
        Some(Commands::Serve {
            port,
            config,
            stdio,
        }) => run_serve(port, config, stdio).await,
        Some(Commands::Models) => run_models(),
        None => {
            if cli.query.is_empty() {
                output::print_json(&output::error(
                    "search",
                    "NO_QUERY",
                    "No query provided. Usage: ai-search \"your query\"",
                ));
                ExitCode::from(1)
            } else {
                run_search(
                    cli.query, cli.mode, cli.model, cli.split, cli.json, cli.human, false,
                )
                .await
            }
        }
    }
}

async fn run_search(
    query: Vec<String>,
    mode: Option<String>,
    model: Option<String>,
    split: Option<u32>,
    json_mode: bool,
    human: bool,
    stdin_mode: bool,
) -> ExitCode {
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            output::print_json(&output::error("search", "CONFIG_ERROR", &error.to_string()));
            return ExitCode::from(2);
        }
    };

    let query_str = if stdin_mode {
        match read_stdin_query() {
            Ok(query) => query,
            Err(error) => {
                output::print_json(&output::error("search", "STDIN_ERROR", &error.to_string()));
                return ExitCode::from(1);
            }
        }
    } else {
        query.join(" ")
    };

    if query_str.trim().is_empty() {
        output::print_json(&output::error("search", "NO_QUERY", "Empty query"));
        return ExitCode::from(1);
    }

    let requested_mode = mode.unwrap_or_else(|| config.default_mode.clone());
    let search_mode = match SearchMode::from_str(&requested_mode) {
        Ok(mode) => mode,
        Err(error) => {
            output::print_json(&output::error("search", "INVALID_MODE", &error.to_string()));
            return ExitCode::from(1);
        }
    };
    let search_engine = match SearchEngine::new(config.clone()) {
        Ok(engine) => engine,
        Err(error) => {
            output::print_json(&output::error("search", "ENGINE_ERROR", &error.to_string()));
            return ExitCode::from(2);
        }
    };
    let model_ref = model.as_deref();
    let split = split.unwrap_or(config.max_split).max(1);

    match search_engine
        .search(&query_str, search_mode, model_ref, split)
        .await
    {
        Ok(result) => {
            let is_tty = atty_stderr();
            if (is_tty && !json_mode) || human {
                eprintln!(
                    "\n[{} · {}] Search: {}\n",
                    result.mode, result.model, result.query
                );
                println!("{}", result.content);
                eprintln!("\nSources: {}", result.providers.join(", "));
                eprintln!("Tokens: {}", result.tokens);
            } else {
                let data = serde_json::to_value(&result).unwrap();
                output::print_json(&output::success("search", data));
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            output::print_json(&output::error(
                "search",
                "SEARCH_FAILED",
                &error.to_string(),
            ));
            ExitCode::from(3)
        }
    }
}

async fn run_serve(port: Option<u16>, config_path: Option<PathBuf>, stdio: bool) -> ExitCode {
    let config = match load_app_config(config_path.as_deref()) {
        Ok(config) => config,
        Err(error) => {
            output::print_json(&output::error("serve", "CONFIG_ERROR", &error.to_string()));
            return ExitCode::from(2);
        }
    };

    if stdio {
        if let Err(error) = mcp::run_stdio(config).await {
            eprintln!("MCP stdio error: {error}");
            return ExitCode::from(2);
        }
    } else {
        let bound_port = port.unwrap_or(config.server_port).max(1);
        let path = config_path.unwrap_or_else(AppConfig::config_path);
        if let Err(error) = server::run(config, bound_port, path).await {
            output::print_json(&output::error("serve", "SERVER_ERROR", &error.to_string()));
            return ExitCode::from(2);
        }
    }

    ExitCode::SUCCESS
}

fn run_models() -> ExitCode {
    output::print_json_pretty(&output::success(
        "models",
        json!({"models": SEARCH_MODELS, "modes": SEARCH_MODES}),
    ));
    ExitCode::SUCCESS
}

fn load_app_config(path: Option<&Path>) -> anyhow::Result<AppConfig> {
    match path {
        Some(path) => AppConfig::load_from(path),
        None => AppConfig::load(),
    }
}

fn read_stdin_query() -> anyhow::Result<String> {
    let input: serde_json::Value = serde_json::from_reader(std::io::stdin().lock())?;
    let query = input
        .get("query")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    Ok(query)
}

fn atty_stderr() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}
