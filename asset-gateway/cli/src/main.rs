mod config;
mod db;
mod error;
mod output;

mod client;
mod core;
mod frontend;
mod providers;
mod server;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "asset-gateway",
    version,
    about = "Universal asset generation gateway"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Gateway URL (for client commands)
    #[arg(
        long,
        global = true,
        env = "ASSET_GATEWAY_URL",
        default_value = "http://localhost:6700"
    )]
    gateway_url: String,

    /// Output format
    #[arg(long, global = true, default_value = "false")]
    human: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server
    Serve {
        /// Host to bind
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Port to bind
        #[arg(long, default_value_t = 6700)]
        port: u16,
        /// PostgreSQL database URL
        #[arg(
            long,
            env = "ASSET_GATEWAY_DATABASE_URL",
            default_value = "postgres://localhost/asset_gateway"
        )]
        database_url: String,
    },

    /// Authentication
    #[command(subcommand)]
    Auth(client::AuthCommands),

    /// Generate assets
    #[command(subcommand)]
    Generate(client::GenerateCommands),

    /// Manage providers
    #[command(subcommand)]
    Provider(client::ProviderCommands),

    /// Manage credentials
    #[command(subcommand)]
    Credential(client::CredentialCommands),

    /// Manage jobs
    #[command(subcommand)]
    Job(client::JobCommands),

    /// Schema introspection
    Describe {
        /// Command to describe (e.g. "generate.image")
        command: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "asset_gateway=info".into()),
        )
        .init();

    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            host,
            port,
            database_url,
        } => {
            server::run(host, port, database_url).await?;
        }
        Commands::Auth(cmd) => {
            client::handle_auth(cmd, &cli.gateway_url).await?;
        }
        Commands::Generate(cmd) => {
            client::handle_generate(cmd, &cli.gateway_url).await?;
        }
        Commands::Provider(cmd) => {
            client::handle_provider(cmd, &cli.gateway_url).await?;
        }
        Commands::Credential(cmd) => {
            client::handle_credential(cmd, &cli.gateway_url).await?;
        }
        Commands::Job(cmd) => {
            client::handle_job(cmd, &cli.gateway_url).await?;
        }
        Commands::Describe { command } => {
            client::handle_describe(command).await?;
        }
    }

    Ok(())
}
