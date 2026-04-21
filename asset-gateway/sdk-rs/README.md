# origin-asset

Rust SDK for the Origin Asset Gateway service.

[![Crates.io](https://img.shields.io/crates/v/origin-asset.svg)](https://crates.io/crates/origin-asset)
[![docs.rs](https://docs.rs/origin-asset/badge.svg)](https://docs.rs/origin-asset)
[![License](https://img.shields.io/crates/l/origin-asset.svg)](LICENSE-MIT)

## Installation

```toml
[dependencies]
origin-asset = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust
use origin_asset::AssetClient;

#[tokio::main]
async fn main() -> origin_asset::Result<()> {
    let client = AssetClient::new("your-api-key");

    let image = client.generate_image("a fire sword", None).await?;
    println!("URL: {:?}", image.output_url);

    let healthy = client.health().await?;
    println!("Healthy: {healthy}");

    Ok(())
}
```

## Configuration

### Default endpoint

`AssetClient::new("your-api-key")` uses `https://asset.origingame.dev`.

### Builder

```rust
let client = origin_asset::AssetClient::builder("your-api-key")
    .base_url("https://my-asset-gateway.example.com")
    .http_client(custom_reqwest_client)
    .build();
```

## Supported Operations

- `generate`
- `generate_image`
- `generate_video`
- `generate_audio`
- `generate_tts`
- `generate_music`
- `generate_model3d`
- `generate_sprite`
- `process`
- `jobs`
- `job_status`
- `providers`
- `health`

## Example

```rust
use origin_asset::{AssetClient, AudioOptions, ImageOptions, TtsOptions};

let client = AssetClient::new("your-api-key");

let image = client.generate_image("a medieval castle", Some(ImageOptions {
    size: Some("1792x1024".into()),
    model: Some("gpt-image-1".into()),
    transparent: Some(true),
    ..Default::default()
})).await?;

let speech = client.generate_tts("Hello world!", Some(TtsOptions {
    voice: Some("alloy".into()),
    ..Default::default()
})).await?;

let sfx = client.generate_audio("explosion boom", Some(AudioOptions {
    duration: Some(3),
    ..Default::default()
})).await?;

let jobs = client.jobs(Some("completed"), Some(10)).await?;
```

## Error Handling

All methods return `origin_asset::Result<T>`, which uses `OriginError`:

```rust
use origin_asset::OriginError;

match origin_asset::AssetClient::new("key").generate_image("test", None).await {
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
