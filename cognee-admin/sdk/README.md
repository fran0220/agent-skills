# origin-gamedb

Rust SDK for Origin GameDB — a standalone client for the Cognee-backed knowledge graph service used to ingest, structure, and search game design data.

[![Crates.io](https://img.shields.io/crates/v/origin-gamedb.svg)](https://crates.io/crates/origin-gamedb)
[![docs.rs](https://docs.rs/origin-gamedb/badge.svg)](https://docs.rs/origin-gamedb)
[![License](https://img.shields.io/crates/l/origin-gamedb.svg)](LICENSE-MIT)

## Installation

```toml
[dependencies]
origin-gamedb = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust,no_run
use origin_gamedb::{CognifyOptions, GameDbClient, SearchOptions, SearchType};

#[tokio::main]
async fn main() -> origin_gamedb::Result<()> {
    let gamedb = GameDbClient::new("ca_xxx_token");

    gamedb.create_dataset("quest-design").await?;
    gamedb
        .add_text(
            "quest-design",
            "Side quests should reveal traversal mechanics before boss encounters.",
        )
        .await?;

    gamedb
        .cognify(&CognifyOptions {
            datasets: Some(vec!["quest-design".into()]),
            run_in_background: Some(true),
            ..Default::default()
        })
        .await?;

    let results = gamedb
        .search(
            "How should traversal tutorials connect to quest pacing?",
            Some(SearchOptions {
                search_type: Some(SearchType::Summaries),
                top_k: Some(5),
                datasets: Some(vec!["quest-design".into()]),
            }),
        )
        .await?;

    println!("{results}");
    Ok(())
}
```

## Configuration

### Default setup

`GameDbClient::new("ca_xxx_token")` uses the default API base URL:

- `https://cogneeapi.origingame.dev`

### Builder

Use the builder to point at a different deployment or inject a custom `reqwest::Client`:

```rust
let gamedb = origin_gamedb::GameDbClient::builder("ca_xxx_token")
    .base_url("https://my-gamedb.example.com")
    .http_client(reqwest::Client::new())
    .build();
```

## What You Can Do

### Dataset management

```rust,no_run
let datasets = gamedb.datasets().await?;
gamedb.create_dataset("encounter-design").await?;
gamedb.delete_dataset("dataset-id").await?;
gamedb.delete_all_datasets().await?;
let status = gamedb.dataset_status().await?;
```

### Add text or files

```rust,no_run
gamedb
    .add_text("encounter-design", "Aggressive enemies should telegraph gap closers.")
    .await?;

gamedb
    .add_file("encounter-design", "./docs/encounters.md")
    .await?;
```

### Build the graph

```rust,no_run
gamedb
    .cognify(&origin_gamedb::CognifyOptions {
        datasets: Some(vec!["encounter-design".into()]),
        run_in_background: Some(true),
        ..Default::default()
    })
    .await?;
```

### Search knowledge

```rust,no_run
let results = gamedb
    .search(
        "What patterns create readable enemy pressure?",
        Some(origin_gamedb::SearchOptions {
            search_type: Some(origin_gamedb::SearchType::Insights),
            top_k: Some(8),
            datasets: Some(vec!["encounter-design".into()]),
        }),
    )
    .await?;
```

## API Surface

`GameDbClient` exposes:

- `health()`
- `health_detailed()`
- `datasets()`
- `create_dataset()`
- `delete_dataset()`
- `delete_all_datasets()`
- `dataset_status()`
- `dataset_graph()`
- `dataset_data()`
- `delete_data()`
- `add_text()`
- `add_file()`
- `cognify()`
- `search()`
- `search_history()`
- `settings()`
- `save_settings()`
- `ontologies()`

## Error Handling

All methods return `origin_gamedb::Result<T>`, which uses `OriginError`:

```rust,no_run
use origin_gamedb::OriginError;

match gamedb.search("boss phase transitions", None).await {
    Ok(results) => println!("{results}"),
    Err(OriginError::Api { status, message, .. }) => {
        eprintln!("API error ({status}): {message}");
    }
    Err(OriginError::Http(err)) => eprintln!("Network error: {err}"),
    Err(err) => eprintln!("Other error: {err}"),
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
