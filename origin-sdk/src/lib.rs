//! # Origin Asset
//!
//! Rust client for the Origin platform services:
//!
//! - **Asset Gateway** — generate images, video, audio, TTS, music, 3D models, sprites, text
//! - **AI Search** — multi-source web search with AI summarization
//! - **Cognee** — knowledge graph construction and semantic search
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use origin_asset::OriginClient;
//!
//! #[tokio::main]
//! async fn main() -> origin_asset::Result<()> {
//!     let client = OriginClient::new("your-api-key");
//!
//!     // Generate an image
//!     let image = client.asset().generate_image("a fire sword", None).await?;
//!     println!("Image URL: {:?}", image.output_url);
//!
//!     // Search the web
//!     let results = client.search().search("Rust async runtime", None).await?;
//!     println!("Content: {}", results.content);
//!
//!     // Search knowledge graph
//!     let knowledge = client.cognee().search("game combat system", None).await?;
//!     println!("{:?}", knowledge);
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod transport;

#[cfg(feature = "asset")]
pub mod asset;
#[cfg(feature = "cognee")]
pub mod cognee;
#[cfg(feature = "search")]
pub mod search;

pub use error::{OriginError, Result};
use transport::HttpTransport;

/// Default base URLs for each service.
pub mod defaults {
    pub const ASSET_GATEWAY_URL: &str = "https://asset.origingame.dev";
    pub const AI_SEARCH_URL: &str = "https://search.xiaomao.chat";
    pub const COGNEE_URL: &str = "https://cogneeapi.origingame.dev";
}

/// Builder for configuring an [`OriginClient`].
pub struct OriginClientBuilder {
    api_key: String,
    asset_url: String,
    search_url: String,
    cognee_url: String,
    cognee_token: Option<String>,
    client: Option<reqwest::Client>,
}

impl OriginClientBuilder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            asset_url: defaults::ASSET_GATEWAY_URL.to_string(),
            search_url: defaults::AI_SEARCH_URL.to_string(),
            cognee_url: defaults::COGNEE_URL.to_string(),
            cognee_token: None,
            client: None,
        }
    }

    /// Override the Asset Gateway base URL.
    pub fn asset_url(mut self, url: impl Into<String>) -> Self {
        self.asset_url = url.into();
        self
    }

    /// Override the AI Search base URL.
    pub fn search_url(mut self, url: impl Into<String>) -> Self {
        self.search_url = url.into();
        self
    }

    /// Override the Cognee API base URL.
    pub fn cognee_url(mut self, url: impl Into<String>) -> Self {
        self.cognee_url = url.into();
        self
    }

    /// Use a separate `ca_xxx` token for Cognee (defaults to main api_key).
    pub fn cognee_token(mut self, token: impl Into<String>) -> Self {
        self.cognee_token = Some(token.into());
        self
    }

    /// Use a custom `reqwest::Client` for all services.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the [`OriginClient`].
    pub fn build(self) -> OriginClient {
        let make_transport = |url: String, key: String| {
            let transport = HttpTransport::new(url, key);
            if let Some(ref client) = self.client {
                transport.with_client(client.clone())
            } else {
                transport
            }
        };

        let cognee_key = self.cognee_token.unwrap_or_else(|| self.api_key.clone());

        OriginClient {
            asset_transport: make_transport(self.asset_url, self.api_key.clone()),
            search_transport: make_transport(self.search_url, self.api_key.clone()),
            cognee_transport: make_transport(self.cognee_url, cognee_key),
        }
    }
}

/// Unified client for all Origin platform services.
///
/// Use [`OriginClient::new`] for defaults or [`OriginClient::builder`] for customization.
#[derive(Debug, Clone)]
pub struct OriginClient {
    asset_transport: HttpTransport,
    search_transport: HttpTransport,
    cognee_transport: HttpTransport,
}

impl OriginClient {
    /// Create a client with default URLs and a shared API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        OriginClientBuilder::new(api_key).build()
    }

    /// Create a builder for fine-grained configuration.
    pub fn builder(api_key: impl Into<String>) -> OriginClientBuilder {
        OriginClientBuilder::new(api_key)
    }

    /// Access the Asset Gateway client.
    #[cfg(feature = "asset")]
    pub fn asset(&self) -> asset::AssetClient {
        asset::AssetClient::new(self.asset_transport.clone())
    }

    /// Access the AI Search client.
    #[cfg(feature = "search")]
    pub fn search(&self) -> search::SearchClient {
        search::SearchClient::new(self.search_transport.clone())
    }

    /// Access the Cognee knowledge graph client.
    #[cfg(feature = "cognee")]
    pub fn cognee(&self) -> cognee::CogneeClient {
        cognee::CogneeClient::new(self.cognee_transport.clone())
    }
}
