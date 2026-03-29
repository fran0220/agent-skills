use clap::{Parser, Subcommand};
use std::path::PathBuf;

use cognee_admin::client;
use cognee_admin::cognee_client::CogneeClient;
use cognee_admin::config::AuthConfig;
use cognee_admin::db;
use cognee_admin::error::AppError;
use cognee_admin::output;
use cognee_admin::server;

#[derive(Parser)]
#[command(
    name = "cognee-admin",
    version,
    about = "Cognee knowledge engine management CLI"
)]
struct Cli {
    /// Cognee API URL
    #[arg(
        long,
        env = "COGNEE_URL",
        default_value = "https://cogneeapi.xiaomao.chat"
    )]
    cognee_url: String,

    /// PostgreSQL connection string
    #[arg(
        long,
        env = "COGNEE_ADMIN_DB",
        default_value = "postgres://cognee:cognee@localhost:5433/cognee_db"
    )]
    db: String,

    /// Cognee JWT for Cognee API authentication
    #[arg(long, env = "COGNEE_JWT")]
    cognee_jwt: Option<String>,

    /// Admin panel token (ca_xxx)
    #[arg(
        long = "admin-token",
        visible_alias = "token",
        env = "COGNEE_ADMIN_TOKEN"
    )]
    admin_token: Option<String>,

    /// Human-readable output
    #[arg(long, default_value = "false")]
    human: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start web management panel
    Serve {
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        #[arg(long, default_value = "9847")]
        port: u16,
    },
    /// Health check
    Health {
        #[arg(long)]
        detailed: bool,
        #[arg(long, default_value_t = false)]
        watch: bool,
        #[arg(long, default_value = "30")]
        interval: u64,
    },
    /// Dataset management
    Dataset {
        #[command(subcommand)]
        action: DatasetAction,
    },
    /// Data operations
    Data {
        #[command(subcommand)]
        action: DataAction,
    },
    /// Trigger knowledge construction
    Cognify {
        #[arg(long)]
        dataset_id: Option<String>,
        #[arg(long)]
        dataset_name: Option<String>,
        #[arg(long)]
        custom_prompt: Option<String>,
        #[arg(long)]
        custom_prompt_file: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        background: bool,
        #[arg(long)]
        chunks_per_batch: Option<u32>,
    },
    /// Search knowledge base
    #[command(args_conflicts_with_subcommands = true)]
    Search {
        query: Option<String>,
        #[arg(long, default_value = "INSIGHTS")]
        search_type: String,
        #[arg(long, default_value = "5")]
        top_k: u32,
        #[arg(long, value_delimiter = ',')]
        datasets: Vec<String>,
        #[arg(long, default_value_t = false)]
        verbose: bool,
        #[command(subcommand)]
        action: Option<SearchAction>,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Request log management
    Log {
        #[command(subcommand)]
        action: LogAction,
    },
    /// Pipeline run history
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },
    /// Command introspection
    Describe { command: Option<String> },
    /// Login to Cognee
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    /// Token management (admin only)
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
    /// Ontology management
    Ontology {
        #[command(subcommand)]
        action: OntologyAction,
    },
}

#[derive(Subcommand)]
enum TokenAction {
    /// Create a new API token
    Create {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "user")]
        role: String,
    },
    /// List all tokens
    List,
    /// Revoke (disable) a token
    Revoke { id: String },
    /// Delete a token permanently
    Delete { id: String },
}

#[derive(Subcommand)]
enum DatasetAction {
    /// List all datasets
    List,
    /// Create a new dataset
    Create {
        /// Dataset name
        name: String,
    },
    /// Delete a dataset
    Delete {
        /// Dataset ID
        id: String,
    },
    /// Delete all datasets
    DeleteAll {
        #[arg(long, default_value_t = false)]
        yes: bool,
    },
    /// Show dataset processing status
    Status,
    /// Show dataset knowledge graph
    Graph {
        /// Dataset ID
        id: String,
    },
}

#[derive(Subcommand)]
enum DataAction {
    /// Add data to a dataset
    Add {
        /// Target dataset name
        #[arg(long)]
        dataset: String,
        /// Text content to add
        content: String,
    },
    /// Upload a file to a dataset
    AddFile {
        /// Target dataset name
        #[arg(long)]
        dataset: String,
        /// File path to upload
        path: String,
    },
    /// Upload all matching files in a directory to a dataset
    AddDir {
        /// Target dataset name
        #[arg(long)]
        dataset: String,
        /// Glob pattern used to filter files
        #[arg(long, default_value = "*")]
        glob: String,
        /// Directory to scan recursively
        dir: String,
    },
    /// List data in a dataset
    List {
        /// Dataset ID
        dataset_id: String,
    },
    /// Delete specific data
    Delete {
        /// Dataset ID
        #[arg(long)]
        dataset_id: String,
        /// Data ID
        #[arg(long)]
        data_id: String,
    },
    /// Get raw data content
    Raw {
        /// Dataset ID
        #[arg(long)]
        dataset_id: String,
        /// Data ID
        #[arg(long)]
        data_id: String,
    },
    /// Replace an existing data item with a new file
    Update {
        /// Dataset ID
        #[arg(long)]
        dataset_id: String,
        /// Data ID
        #[arg(long)]
        data_id: String,
        /// File path to upload
        path: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get current settings
    Get,
    /// Update settings
    Set {
        /// JSON settings string
        settings: String,
    },
}

#[derive(Subcommand)]
enum SearchAction {
    /// Show search history
    History,
}

#[derive(Subcommand)]
enum LogAction {
    /// List recent request logs
    List {
        /// Maximum number of logs to return
        #[arg(long, default_value = "50")]
        limit: i64,
        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: i64,
    },
    /// Show request log statistics
    Stats,
}

#[derive(Subcommand)]
enum PipelineAction {
    /// List recent pipeline runs
    List {
        /// Maximum number of runs to return
        #[arg(long, default_value = "20")]
        limit: i64,
    },
    /// Show pipeline run details
    Detail {
        /// Pipeline run ID
        id: String,
    },
}

#[derive(Subcommand)]
enum OntologyAction {
    /// Upload an ontology file
    Upload {
        #[arg(long)]
        key: String,
        file: PathBuf,
    },
    /// List uploaded ontologies
    List,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let cli = Cli::parse();
    let human = cli.human;
    let auth = AuthConfig::load();

    let cognee_jwt = cli.cognee_jwt.clone().or_else(|| auth.cognee_jwt.clone());
    let _admin_token = cli.admin_token.clone().or_else(|| auth.admin_token.clone());

    // Connect to PostgreSQL and run migrations
    let pool = match db::connect(&cli.db).await {
        Ok(pool) => {
            if let Err(e) = db::run_migrations(&pool).await {
                tracing::warn!("Migration warning: {}", e);
            }
            Some(pool)
        }
        Err(e) => {
            tracing::warn!("Database unavailable (logging disabled): {}", e);
            None
        }
    };

    // Build the Cognee HTTP client
    let mut cognee = CogneeClient::new(&cli.cognee_url);
    if matches!(
        &cli.command,
        Commands::Health { .. }
            | Commands::Dataset { .. }
            | Commands::Data { .. }
            | Commands::Cognify { .. }
            | Commands::Search { .. }
            | Commands::Ontology { .. }
            | Commands::Config { .. }
    ) {
        if let Some(jwt) = cognee_jwt {
            cognee = cognee.with_token(jwt);
        }
    }
    if let Some(p) = pool.clone() {
        cognee = cognee.with_pool(p);
    }

    match cli.command {
        Commands::Serve { host, port } => {
            let Some(p) = pool else {
                output::print_result(
                    "serve",
                    Err(AppError::Config(
                        "Database connection required for web server".into(),
                    )),
                    human,
                );
                return;
            };
            if let Err(e) = server::run(&host, port, &cli.cognee_url, p).await {
                output::print_result("serve", Err(AppError::Internal(e)), human);
            }
        }

        Commands::Health {
            detailed,
            watch,
            interval,
        } => {
            let result = client::health_cmd::run(&cognee, detailed, watch, interval, human).await;
            output::print_result("health", result, human);
        }

        Commands::Login { username, password } => {
            let result = client::login_cmd::run(&cognee, &username, &password).await;
            output::print_result("login", result, human);
        }

        Commands::Dataset { action } => match action {
            DatasetAction::List => {
                let result = client::dataset_cmd::list(&cognee).await;
                output::print_result("dataset.list", result, human);
            }
            DatasetAction::Create { name } => {
                let result = client::dataset_cmd::create(&cognee, &name).await;
                output::print_result("dataset.create", result, human);
            }
            DatasetAction::Delete { id } => {
                let result = client::dataset_cmd::delete(&cognee, &id).await;
                output::print_result("dataset.delete", result, human);
            }
            DatasetAction::DeleteAll { yes } => {
                let result = client::dataset_cmd::delete_all(&cognee, yes).await;
                output::print_result("dataset.delete-all", result, human);
            }
            DatasetAction::Status => {
                let result = client::dataset_cmd::status(&cognee).await;
                output::print_result("dataset.status", result, human);
            }
            DatasetAction::Graph { id } => {
                let result = client::dataset_cmd::graph(&cognee, &id).await;
                output::print_result("dataset.graph", result, human);
            }
        },

        Commands::Data { action } => match action {
            DataAction::Add { dataset, content } => {
                let result = client::data_cmd::add(&cognee, &dataset, &content).await;
                output::print_result("data.add", result, human);
            }
            DataAction::AddFile { dataset, path } => {
                let result = client::data_cmd::add_file(&cognee, &dataset, &path).await;
                output::print_result("data.add-file", result, human);
            }
            DataAction::AddDir { dataset, glob, dir } => {
                let result = client::data_cmd::add_dir(&cognee, &dataset, &dir, &glob).await;
                output::print_result("data.add-dir", result, human);
            }
            DataAction::List { dataset_id } => {
                let result = client::data_cmd::list(&cognee, &dataset_id).await;
                output::print_result("data.list", result, human);
            }
            DataAction::Delete {
                dataset_id,
                data_id,
            } => {
                let result = client::data_cmd::delete(&cognee, &dataset_id, &data_id).await;
                output::print_result("data.delete", result, human);
            }
            DataAction::Raw {
                dataset_id,
                data_id,
            } => {
                let result = client::data_cmd::raw(&cognee, &dataset_id, &data_id).await;
                output::print_result("data.raw", result, human);
            }
            DataAction::Update {
                dataset_id,
                data_id,
                path,
            } => {
                let result = client::data_cmd::update(&cognee, &dataset_id, &data_id, &path).await;
                output::print_result("data.update", result, human);
            }
        },

        Commands::Cognify {
            dataset_id,
            dataset_name,
            custom_prompt,
            custom_prompt_file,
            background,
            chunks_per_batch,
        } => {
            let result = client::cognify_cmd::run(
                &cognee,
                dataset_id.as_deref(),
                dataset_name.as_deref(),
                custom_prompt.as_deref(),
                custom_prompt_file.as_deref(),
                background,
                chunks_per_batch,
            )
            .await;
            output::print_result("cognify", result, human);
        }

        Commands::Search {
            query,
            search_type,
            top_k,
            datasets,
            verbose,
            action,
        } => match action {
            Some(SearchAction::History) => {
                let result = client::search_cmd::history(&cognee).await;
                output::print_result("search.history", result, human);
            }
            None => {
                let Some(query) = query else {
                    output::print_result(
                        "search",
                        Err(AppError::Config(
                            "query is required unless using `search history`".into(),
                        )),
                        human,
                    );
                    return;
                };

                let result = client::search_cmd::run(
                    &cognee,
                    &query,
                    &search_type,
                    top_k,
                    &datasets,
                    verbose,
                )
                .await;
                output::print_result("search", result, human);
            }
        },

        Commands::Config { action } => match action {
            ConfigAction::Get => {
                let result = client::config_cmd::get(&cognee).await;
                output::print_result("config.get", result, human);
            }
            ConfigAction::Set { settings } => {
                let result = client::config_cmd::set(&cognee, &settings).await;
                output::print_result("config.set", result, human);
            }
        },

        Commands::Log { action } => {
            let Some(ref p) = pool else {
                output::print_result(
                    "log",
                    Err(AppError::Config(
                        "Database connection required for log queries".into(),
                    )),
                    human,
                );
                return;
            };
            match action {
                LogAction::List { limit, offset } => {
                    let result = client::log_cmd::list(p, limit, offset).await;
                    output::print_result("log.list", result, human);
                }
                LogAction::Stats => {
                    let result = client::log_cmd::stats(p).await;
                    output::print_result("log.stats", result, human);
                }
            }
        }

        Commands::Pipeline { action } => {
            let Some(ref p) = pool else {
                output::print_result(
                    "pipeline",
                    Err(AppError::Config(
                        "Database connection required for pipeline queries".into(),
                    )),
                    human,
                );
                return;
            };
            match action {
                PipelineAction::List { limit } => {
                    let result = client::pipeline_cmd::list(p, limit).await;
                    output::print_result("pipeline.list", result, human);
                }
                PipelineAction::Detail { id } => {
                    let result = client::pipeline_cmd::detail(p, &id).await;
                    output::print_result("pipeline.detail", result, human);
                }
            }
        }

        Commands::Describe { command } => {
            let result = client::describe_cmd::run(command.as_deref());
            output::print_result("describe", Ok(result), human);
        }

        Commands::Token { action } => {
            let Some(ref p) = pool else {
                output::print_result(
                    "token",
                    Err(AppError::Config(
                        "Database connection required for token management".into(),
                    )),
                    human,
                );
                return;
            };
            match action {
                TokenAction::Create { name, role } => {
                    let result = client::token_cmd::create(p, &name, &role).await;
                    output::print_result("token.create", result, human);
                }
                TokenAction::List => {
                    let result = client::token_cmd::list(p).await;
                    output::print_result("token.list", result, human);
                }
                TokenAction::Revoke { id } => {
                    let result = client::token_cmd::revoke(p, &id).await;
                    output::print_result("token.revoke", result, human);
                }
                TokenAction::Delete { id } => {
                    let result = client::token_cmd::delete(p, &id).await;
                    output::print_result("token.delete", result, human);
                }
            }
        }

        Commands::Ontology { action } => match action {
            OntologyAction::Upload { key, file } => {
                let result = client::ontology_cmd::upload(&cognee, &key, &file).await;
                output::print_result("ontology.upload", result, human);
            }
            OntologyAction::List => {
                let result = client::ontology_cmd::list(&cognee).await;
                output::print_result("ontology.list", result, human);
            }
        },
    }
}
