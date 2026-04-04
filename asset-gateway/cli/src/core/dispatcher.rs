use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::registry::ProviderRegistry;
use super::{AssetProvider, GenerateRequest, GenerateResponse, HealthStatus};

const HEALTH_CACHE_TTL: Duration = Duration::from_secs(60);

/// Dispatcher routes generation requests to the best provider
/// based on requested asset type, provider health, and routing strategy.
pub struct Dispatcher {
    registry: Arc<ProviderRegistry>,
    health_cache: RwLock<HashMap<String, (Instant, HealthStatus)>>,
}

impl Dispatcher {
    pub fn new(registry: Arc<ProviderRegistry>) -> Self {
        Self {
            registry,
            health_cache: RwLock::new(HashMap::new()),
        }
    }

    #[allow(dead_code)]
    pub async fn invalidate_health_cache(&self) {
        self.health_cache.write().await.clear();
    }

    /// Route a request to candidate providers in priority order and use fallback
    /// when a higher-priority provider fails.
    pub async fn dispatch(
        &self,
        req: &GenerateRequest,
        provider_hint: Option<&str>,
    ) -> anyhow::Result<GenerateResponse> {
        let candidates = self.select_providers(req, provider_hint).await?;
        let mut failures = Vec::new();

        for provider in candidates {
            tracing::info!(
                provider = provider.id(),
                asset_type = req.asset_type.as_str(),
                "dispatching generate request"
            );

            match provider.generate(req).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    tracing::error!(
                        provider = provider.id(),
                        error = %err,
                        "provider failed, trying fallback"
                    );
                    failures.push(format!("{}: {}", provider.id(), err));
                }
            }
        }

        anyhow::bail!(
            "all providers failed for asset type {}: {}",
            req.asset_type,
            failures.join(" | ")
        )
    }

    async fn select_providers(
        &self,
        req: &GenerateRequest,
        hint: Option<&str>,
    ) -> anyhow::Result<Vec<Arc<dyn AssetProvider>>> {
        if let Some(id) = hint {
            let provider = self
                .registry
                .get(id)
                .await
                .ok_or_else(|| anyhow::anyhow!("provider not found: {}", id))?;

            if !provider.asset_types().contains(&req.asset_type) {
                anyhow::bail!(
                    "provider {} does not support asset type {}",
                    provider.id(),
                    req.asset_type
                );
            }

            let healthy = self.is_provider_healthy(provider.as_ref()).await;
            if !healthy {
                anyhow::bail!("provider {} is unhealthy", provider.id());
            }

            return Ok(vec![provider]);
        }

        let candidates = self.registry.find_by_type(req.asset_type).await;
        if candidates.is_empty() {
            anyhow::bail!("no provider available for asset type: {}", req.asset_type);
        }

        let mut healthy_candidates = Vec::new();
        for provider in candidates {
            if self.is_provider_healthy(provider.as_ref()).await {
                healthy_candidates.push(provider);
            }
        }

        if healthy_candidates.is_empty() {
            anyhow::bail!(
                "no healthy provider available for asset type: {}",
                req.asset_type
            );
        }

        healthy_candidates.sort_by(|a, b| {
            let score_a = Self::provider_score(a.as_ref(), req);
            let score_b = Self::provider_score(b.as_ref(), req);
            score_b.cmp(&score_a).then_with(|| a.id().cmp(b.id()))
        });

        Ok(healthy_candidates)
    }

    async fn is_provider_healthy(&self, provider: &dyn AssetProvider) -> bool {
        let id = provider.id().to_string();

        // Check cache (read lock)
        {
            let cache = self.health_cache.read().await;
            if let Some((ts, status)) = cache.get(&id) {
                if ts.elapsed() < HEALTH_CACHE_TTL {
                    return status.healthy;
                }
            }
        }

        // Cache miss or expired — perform real health check
        let result = provider.health_check().await;
        let healthy = match &result {
            Ok(status) if status.healthy => true,
            Ok(status) => {
                tracing::error!(
                    provider = provider.id(),
                    latency_ms = status.latency_ms,
                    message = ?status.message,
                    "provider marked unhealthy"
                );
                false
            }
            Err(err) => {
                tracing::error!(
                    provider = provider.id(),
                    error = %err,
                    "provider health check failed"
                );
                false
            }
        };

        // Store in cache (write lock)
        if let Ok(status) = result {
            self.health_cache
                .write()
                .await
                .insert(id, (Instant::now(), status));
        }

        healthy
    }

    fn provider_score(provider: &dyn AssetProvider, _req: &GenerateRequest) -> i32 {
        provider.capabilities().priority
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::core::{AssetType, GenerateRequest, HealthStatus, ProviderCapabilities};

    struct MockProvider {
        id: &'static str,
        asset_types: &'static [AssetType],
        caps: ProviderCapabilities,
        healthy: bool,
        fail_generate: bool,
    }

    #[async_trait::async_trait]
    impl AssetProvider for MockProvider {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn id(&self) -> &str {
            self.id
        }

        fn display_name(&self) -> &str {
            self.id
        }

        fn asset_types(&self) -> &[AssetType] {
            self.asset_types
        }

        fn capabilities(&self) -> ProviderCapabilities {
            self.caps.clone()
        }

        async fn generate(&self, _req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
            if self.fail_generate {
                anyhow::bail!("forced failure")
            }

            Ok(GenerateResponse {
                provider_id: self.id.to_string(),
                output_path: None,
                output_url: None,
                output_data: Some("ok".to_string()),
                metadata: json!({}),
                cost_usd: None,
                elapsed_ms: 1,
            })
        }

        async fn health_check(&self) -> anyhow::Result<HealthStatus> {
            Ok(HealthStatus {
                healthy: self.healthy,
                latency_ms: Some(1),
                message: None,
            })
        }
    }

    #[tokio::test]
    async fn image_routes_to_gemini() {
        let registry = Arc::new(ProviderRegistry::new());

        registry
            .register(Arc::new(MockProvider {
                id: "gemini_image",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    priority: 100,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        registry
            .register(Arc::new(MockProvider {
                id: "grok_image",
                asset_types: &[AssetType::Video],
                caps: ProviderCapabilities {
                    priority: 100,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        let dispatcher = Dispatcher::new(registry);
        let req = GenerateRequest {
            asset_type: AssetType::Image,
            prompt: Some("hero image".to_string()),
            model: None,
            input_file: None,
            reference_images: vec![],
            edit_mode: None,
            session_id: None,
            params: json!({}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "gemini_image");
    }

    #[tokio::test]
    async fn video_routes_to_grok() {
        let registry = Arc::new(ProviderRegistry::new());

        registry
            .register(Arc::new(MockProvider {
                id: "gemini_image",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    priority: 100,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        registry
            .register(Arc::new(MockProvider {
                id: "grok_image",
                asset_types: &[AssetType::Video],
                caps: ProviderCapabilities {
                    priority: 100,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        let dispatcher = Dispatcher::new(registry);
        let req = GenerateRequest {
            asset_type: AssetType::Video,
            prompt: Some("a running cat".to_string()),
            model: None,
            input_file: None,
            reference_images: vec![],
            edit_mode: None,
            session_id: None,
            params: json!({}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "grok_image");
    }

    #[tokio::test]
    async fn music_routes_to_elevenlabs() {
        let registry = Arc::new(ProviderRegistry::new());

        registry
            .register(Arc::new(MockProvider {
                id: "elevenlabs",
                asset_types: &[AssetType::Audio, AssetType::Music],
                caps: ProviderCapabilities {
                    priority: 100,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        let dispatcher = Dispatcher::new(registry);
        let req = GenerateRequest {
            asset_type: AssetType::Music,
            prompt: Some("warm ambient synth with slow build".to_string()),
            model: None,
            input_file: None,
            reference_images: vec![],
            edit_mode: None,
            session_id: None,
            params: json!({}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "elevenlabs");
    }

    #[tokio::test]
    async fn skips_unhealthy_provider() {
        let registry = Arc::new(ProviderRegistry::new());

        registry
            .register(Arc::new(MockProvider {
                id: "primary",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    priority: 100,
                    ..Default::default()
                },
                healthy: false,
                fail_generate: false,
            }))
            .await;

        registry
            .register(Arc::new(MockProvider {
                id: "fallback",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    priority: 5,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        let dispatcher = Dispatcher::new(registry);
        let req = GenerateRequest {
            asset_type: AssetType::Image,
            prompt: Some("portrait".to_string()),
            model: None,
            input_file: None,
            reference_images: vec![],
            edit_mode: None,
            session_id: None,
            params: json!({}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "fallback");
    }
}
