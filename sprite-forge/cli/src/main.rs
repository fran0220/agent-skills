mod client;
mod config;
mod db;
mod migrations;
mod pipeline;
mod server;
mod types;

use anyhow::{Result, anyhow};
use axum::http::{self, HeaderValue, Method};
use clap::{Parser, Subcommand, ValueEnum};
use config::SpriteForgeConfig;
use std::{path::PathBuf, sync::Arc};
use tokio::fs;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use types::{AnimationType, Direction, GridSize, SpriteRequest};

#[derive(Debug, Parser)]
#[command(name = "sprite-forge")]
#[command(about = "Sprite generation service and CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Serve,
    Generate(GenerateArgs),
}

#[derive(Debug, clap::Args)]
struct GenerateArgs {
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long)]
    input: Option<String>,
    #[arg(long, value_enum, default_value_t = AnimationArg::Walk)]
    animation: AnimationArg,
    #[arg(long, value_enum, default_value_t = GridArg::Grid3x3)]
    grid: GridArg,
    #[arg(long, value_enum, default_value_t = DirectionArg::Right)]
    direction: DirectionArg,
    #[arg(long)]
    style: Option<String>,
    #[arg(long, default_value = "./output")]
    output_dir: PathBuf,
}

#[derive(Debug, Clone, ValueEnum)]
enum AnimationArg {
    Walk,
    Run,
    Attack,
    Idle,
    Death,
}

#[derive(Debug, Clone, ValueEnum)]
enum GridArg {
    #[value(name = "2x2")]
    Grid2x2,
    #[value(name = "3x3")]
    Grid3x3,
    #[value(name = "4x4")]
    Grid4x4,
}

#[derive(Debug, Clone, ValueEnum)]
enum DirectionArg {
    Right,
    Left,
    Front,
    Back,
}

impl From<AnimationArg> for AnimationType {
    fn from(value: AnimationArg) -> Self {
        match value {
            AnimationArg::Walk => Self::Walk,
            AnimationArg::Run => Self::Run,
            AnimationArg::Attack => Self::Attack,
            AnimationArg::Idle => Self::Idle,
            AnimationArg::Death => Self::Death,
        }
    }
}

impl From<GridArg> for GridSize {
    fn from(value: GridArg) -> Self {
        match value {
            GridArg::Grid2x2 => Self::Grid2x2,
            GridArg::Grid3x3 => Self::Grid3x3,
            GridArg::Grid4x4 => Self::Grid4x4,
        }
    }
}

impl From<DirectionArg> for Direction {
    fn from(value: DirectionArg) -> Self {
        match value {
            DirectionArg::Right => Self::Right,
            DirectionArg::Left => Self::Left,
            DirectionArg::Front => Self::Front,
            DirectionArg::Back => Self::Back,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Serve => serve().await,
        Commands::Generate(args) => generate(args).await,
    }
}

async fn serve() -> Result<()> {
    let config = SpriteForgeConfig::from_env()?;
    let bind_addr = format!("{}:{}", config.host, config.port);
    let db = db::init_database(&config.database_path).await?;
    let state = Arc::new(server::state::AppState::new(config, db));
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://localhost:5174"),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([http::header::AUTHORIZATION, http::header::CONTENT_TYPE]);
    let app = server::routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("sprite-forge server listening on {}", bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn generate(args: GenerateArgs) -> Result<()> {
    if args.prompt.is_none() && args.input.is_none() {
        return Err(anyhow!(
            "At least one of --prompt or --input must be provided"
        ));
    }

    let config = SpriteForgeConfig::from_env()?;
    let llm = client::llm::LlmClient::new(
        &config.llm_proxy_url,
        &config.llm_proxy_key,
        &config.llm_model,
    );
    let gemini = client::gemini::GeminiClient::new(
        &config.gemini_base_url,
        &config.gemini_api_key,
        &config.gemini_model,
    );

    let request = SpriteRequest {
        character_description: args.prompt,
        character_image: args.input,
        animation_type: args.animation.into(),
        direction: args.direction.into(),
        grid_size: args.grid.into(),
        style: args.style,
        remove_background: true,
    };

    let result = pipeline::run_pipeline(&llm, &gemini, &request).await?;

    fs::create_dir_all(&args.output_dir).await?;

    let grid_path = args.output_dir.join("grid.png");
    fs::write(&grid_path, &result.grid_image).await?;

    let sprite_sheet_path = args.output_dir.join("sprite_sheet.png");
    fs::write(&sprite_sheet_path, &result.sprite_sheet).await?;

    for (index, frame) in result.frames.iter().enumerate() {
        let frame_path = args.output_dir.join(format!("frame_{index:02}.png"));
        fs::write(frame_path, frame).await?;
    }

    let prompt_path = args.output_dir.join("enhanced_prompt.txt");
    fs::write(prompt_path, result.enhanced_prompt.as_bytes()).await?;

    info!("saved generated assets to {}", args.output_dir.display());
    Ok(())
}
