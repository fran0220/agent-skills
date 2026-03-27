use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{AssetProvider, AssetType};

/// Thread-safe registry of all available providers.
pub struct ProviderRegistry {
    providers: RwLock<HashMap<String, Arc<dyn AssetProvider>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, provider: Arc<dyn AssetProvider>) {
        let id = provider.id().to_string();
        self.providers.write().await.insert(id, provider);
    }

    pub async fn unregister(&self, id: &str) -> bool {
        self.providers.write().await.remove(id).is_some()
    }

    pub async fn get(&self, id: &str) -> Option<Arc<dyn AssetProvider>> {
        self.providers.read().await.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<Arc<dyn AssetProvider>> {
        self.providers.read().await.values().cloned().collect()
    }

    /// Find all providers that support a given asset type.
    pub async fn find_by_type(&self, asset_type: AssetType) -> Vec<Arc<dyn AssetProvider>> {
        self.providers
            .read()
            .await
            .values()
            .filter(|p| p.asset_types().contains(&asset_type))
            .cloned()
            .collect()
    }
}
