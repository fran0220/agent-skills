#![recursion_limit = "256"]

mod config;
mod core;
mod db;
mod error;
mod frontend;
mod providers;
mod server;
mod storage;
mod vertex_auth;

use std::path::PathBuf;

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
            long = "db",
            env = "ASSET_GATEWAY_DATABASE_URL",
            default_value = "postgres://localhost/asset_gateway"
        )]
        database_url: String,
        /// Config file path (TOML)
        #[arg(long, env = "ASSET_GATEWAY_CONFIG", default_value = "config.toml")]
        config: PathBuf,
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
            config,
        } => {
            server::run(host, port, database_url, &config).await?;
        }
    }

    Ok(())
}
