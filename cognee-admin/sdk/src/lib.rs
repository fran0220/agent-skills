//! # Origin GameDB
//!
//! Rust SDK for the Origin GameDB service, a knowledge graph API built on top
//! of Cognee for ingesting game design data and querying it semantically.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use origin_gamedb::{CognifyOptions, GameDbClient, SearchOptions, SearchType};
//!
//! #[tokio::main]
//! async fn main() -> origin_gamedb::Result<()> {
//!     let gamedb = GameDbClient::new("ca_xxx_token");
//!
//!     gamedb.create_dataset("combat-design").await?;
//!     gamedb
//!         .add_text("combat-design", "Poise breaks after repeated heavy attacks.")
//!         .await?;
//!
//!     gamedb
//!         .cognify(&CognifyOptions {
//!             datasets: Some(vec!["combat-design".into()]),
//!             run_in_background: Some(true),
//!             ..Default::default()
//!         })
//!         .await?;
//!
//!     let results = gamedb
//!         .search(
//!             "What systems interact with poise?",
//!             Some(SearchOptions {
//!                 search_type: Some(SearchType::Summaries),
//!                 top_k: Some(5),
//!                 ..Default::default()
//!             }),
//!         )
//!         .await?;
//!
//!     println!("{results}");
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod transport;
pub mod types;

pub use client::GameDbClient;
pub use error::{OriginError, Result};
pub use types::*;

/// Default GameDB API endpoint.
pub const DEFAULT_BASE_URL: &str = "https://cogneeapi.origingame.dev";
