# origin-search

Rust SDK for the Origin AI Search service.

[![Crates.io](https://img.shields.io/crates/v/origin-search.svg)](https://crates.io/crates/origin-search)
[![docs.rs](https://docs.rs/origin-search/badge.svg)](https://docs.rs/origin-search)
[![License](https://img.shields.io/crates/l/origin-search.svg)](LICENSE-MIT)

## Installation

```toml
[dependencies]
origin-search = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust
use origin_search::SearchClient;

#[tokio::main]
async fn main() -> origin_search::Result<()> {
    let client = SearchClient::new("your-api-key");

    let result = client.search_fast("Rust async runtime").await?;
    println!("{}", result.content);

    Ok(())
}
```

## Configuration

### Default endpoint

```rust
let client = SearchClient::new("your-api-key");
```

This targets `https://search.xiaomao.chat` by default.

### Custom base URL

```rust
let client = SearchClient::builder("your-api-key")
    .base_url("https://my-search-server.example.com")
    .build();
```

## Usage

### Fast search

```rust
let result = client.search_fast("latest Rust release").await?;
println!("{}", result.content);
```

### Deep multi-source search

```rust
let result = client
    .search_deep("compare React vs Svelte performance")
    .await?;

println!("providers: {:?}", result.providers);
```

### AI answer mode

```rust
let result = client
    .search_answer("what is the capital of France?")
    .await?;

println!("answer: {:?}", result.answer);
```

### Custom search options

```rust
use origin_search::{SearchMode, SearchOptions};

let result = client.search(
    "Rust web frameworks",
    Some(SearchOptions {
        mode: Some(SearchMode::Fast),
        model: Some("grok-4.1-fast".into()),
        split: Some(3),
        num: None,
    }),
).await?;
```

### Other endpoints

```rust
let models = client.models().await?;
let providers = client.providers().await?;
let healthy = client.health().await?;
```

## API Surface

- `search()`
- `search_fast()`
- `search_deep()`
- `search_answer()`
- `models()`
- `providers()`
- `health()`

## Error Handling

All methods return `origin_search::Result<T>`, which uses `OriginError`:

```rust
use origin_search::OriginError;

match client.search_fast("Rust").await {
    Ok(response) => println!("Success: {}", response.content),
    Err(OriginError::Api { status, message, .. }) => {
        eprintln!("API error ({}): {}", status, message);
    }
    Err(OriginError::Http(error)) => eprintln!("Network error: {error}"),
    Err(error) => eprintln!("Other error: {error}"),
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
