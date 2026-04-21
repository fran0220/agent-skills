//! Rust SDK for the Origin AI Search service.
//!
//! # Quick Start
//!
//! ```no_run
//! use origin_search::SearchClient;
//!
//! #[tokio::main]
//! async fn main() -> origin_search::Result<()> {
//!     let client = SearchClient::new("your-api-key");
//!     let result = client.search_fast("Rust async runtime").await?;
//!     println!("{}", result.content);
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod transport;
pub mod types;

pub use client::SearchClient;
pub use error::{OriginError, Result};
pub use types::*;
