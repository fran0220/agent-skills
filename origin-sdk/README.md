# origin-asset

Rust SDK for the Origin platform — unified access to asset generation, AI search, and knowledge graph services.

[![Crates.io](https://img.shields.io/crates/v/origin-asset.svg)](https://crates.io/crates/origin-asset)
[![docs.rs](https://docs.rs/origin-asset/badge.svg)](https://docs.rs/origin-asset)
[![License](https://img.shields.io/crates/l/origin-asset.svg)](LICENSE-MIT)

## Services

| Service | Description | Default Endpoint |
|---------|-------------|------------------|
| **Asset Gateway** | Generate images, video, audio, TTS, music, 3D models, sprites | `https://asset.origingame.dev` |
| **AI Search** | Multi-source web search with AI summarization (Grok + Exa + Tavily) | `https://search.xiaomao.chat` |
| **Cognee** | Knowledge graph construction and semantic search | `https://cogneeapi.origingame.dev` |

## Installation

```toml
[dependencies]
origin-asset = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Or with feature flags to include only what you need:

```toml
[dependencies]
origin-asset = { version = "0.1", default-features = false, features = ["asset", "search"] }
```

## Quick Start

```rust
use origin_asset::OriginClient;

#[tokio::main]
async fn main() -> origin_asset::Result<()> {
    let client = OriginClient::new("your-api-key");

    // Generate an image
    let image = client.asset().generate_image("a fire sword", None).await?;
    println!("URL: {:?}", image.output_url);

    // Search the web
    let results = client.search().search_fast("Rust async runtime").await?;
    println!("{}", results.content);

    // Query knowledge graph
    let knowledge = client.cognee().search("game combat system", None).await?;
    println!("{knowledge}");

    Ok(())
}
```

## Configuration

### Default (shared API key)

```rust
let client = OriginClient::new("your-api-key");
```

### Builder (custom URLs / separate tokens)

```rust
let client = OriginClient::builder("your-api-key")
    .asset_url("https://my-asset-server.example.com")
    .search_url("https://my-search-server.example.com")
    .cognee_url("https://my-cognee-server.example.com")
    .cognee_token("ca_xxx_separate_token")  // Cognee uses ca_xxx tokens
    .http_client(custom_reqwest_client)      // Custom timeouts, proxy, etc.
    .build();
```

## Asset Gateway

Generate images, video, audio, TTS, music, 3D models, and sprites.

```rust
use origin_asset::asset::{ImageOptions, TtsOptions, AudioOptions};

let asset = client.asset();

// Image with options
let image = asset.generate_image("a medieval castle", Some(ImageOptions {
    size: Some("1792x1024".into()),
    model: Some("gpt-image-1".into()),
    transparent: Some(true),
    ..Default::default()
})).await?;

// Text-to-speech
let speech = asset.generate_tts("Hello world!", Some(TtsOptions {
    voice: Some("alloy".into()),
    ..Default::default()
})).await?;

// Sound effects
let sfx = asset.generate_audio("explosion boom", Some(AudioOptions {
    duration: Some(3),
    ..Default::default()
})).await?;

// Video
let video = asset.generate_video("a cat walking", None).await?;

// Music
let music = asset.generate_music("epic orchestral battle theme", None).await?;

// 3D model
let model = asset.generate_model3d("a treasure chest", None).await?;

// Sprite animation
let sprite = asset.generate_sprite("a knight walking cycle", None).await?;

// List providers
let providers = asset.providers().await?;

// Check job status
let jobs = asset.jobs(Some("completed"), Some(10)).await?;
```

## AI Search

Multi-source web search with AI-powered summarization.

```rust
use origin_asset::search::{SearchOptions, SearchMode};

let search = client.search();

// Quick search
let result = search.search_fast("latest Rust release").await?;
println!("{}", result.content);

// Deep multi-pass search
let result = search.search_deep("compare React vs Svelte performance").await?;

// AI answer mode
let result = search.search_answer("what is the capital of France?").await?;
println!("{:?}", result.answer);

// Custom options
let result = search.search("Rust web frameworks", Some(SearchOptions {
    mode: Some(SearchMode::Fast),
    model: Some("grok-4.1-fast".into()),
    split: Some(3),
    num: None,
})).await?;

// List available models
let models = search.models().await?;
```

## Cognee (Knowledge Graph)

Build and query knowledge graphs from your documents.

```rust
use origin_asset::cognee::{CognifyOptions, SearchOptions, SearchType};

let cognee = client.cognee();

// Add text to a dataset
cognee.add_text("my-docs", "Rust is a systems programming language.").await?;

// Upload a file
cognee.add_file("my-docs", "/path/to/document.pdf").await?;

// Build knowledge graph
cognee.cognify(&CognifyOptions {
    datasets: Some(vec!["my-docs".into()]),
    run_in_background: Some(true),
    ..Default::default()
}).await?;

// Search knowledge
let result = cognee.search("What is Rust?", Some(SearchOptions {
    search_type: Some(SearchType::Summaries),
    top_k: Some(5),
    ..Default::default()
})).await?;

// Dataset management
let datasets = cognee.datasets().await?;
cognee.create_dataset("new-dataset").await?;
cognee.delete_dataset("dataset-id").await?;
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `asset` | ✅ | Asset Gateway client |
| `search` | ✅ | AI Search client |
| `cognee` | ✅ | Cognee knowledge graph client |

## Error Handling

All methods return `origin_asset::Result<T>`, which uses `OriginError`:

```rust
use origin_asset::OriginError;

match client.asset().generate_image("test", None).await {
    Ok(response) => println!("Success: {:?}", response.output_url),
    Err(OriginError::Api { status, message, .. }) => {
        eprintln!("API error ({}): {}", status, message);
    }
    Err(OriginError::Http(e)) => eprintln!("Network error: {e}"),
    Err(e) => eprintln!("Other error: {e}"),
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
