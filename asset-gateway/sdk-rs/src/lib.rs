//! # Origin Asset
//!
//! Rust client for the Origin Asset Gateway service.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use origin_asset::AssetClient;
//!
//! #[tokio::main]
//! async fn main() -> origin_asset::Result<()> {
//!     let client = AssetClient::new("your-api-key");
//!
//!     let image = client.generate_image("a fire sword", None).await?;
//!     println!("Image URL: {:?}", image.output_url);
//!
//!     let healthy = client.health().await?;
//!     println!("Healthy: {healthy}");
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod transport;
pub mod types;

pub use client::{AssetClient, AssetClientBuilder, DEFAULT_BASE_URL};
pub use error::{OriginError, Result};
pub use types::*;
