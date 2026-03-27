use std::sync::Arc;

use super::registry::ProviderRegistry;
use super::{AssetProvider, AssetType, GenerateRequest, GenerateResponse};

/// Dispatcher routes generation requests to the best provider
/// based on requested asset type, provider health, and routing strategy.
pub struct Dispatcher {
    registry: Arc<ProviderRegistry>,
}

impl Dispatcher {
    pub fn new(registry: Arc<ProviderRegistry>) -> Self {
        Self { registry }
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
        match provider.health_check().await {
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
        }
    }

    fn provider_score(provider: &dyn AssetProvider, req: &GenerateRequest) -> i32 {
        let caps = provider.capabilities();
        let mut score = caps.priority;

        if req.asset_type == AssetType::Image {
            if req.transparent() {
                if provider.id() == "gpt_image" {
                    score += 10_000;
                } else if caps.supports_transparency {
                    score += 5_000;
                }
            } else if provider.id() == "gemini_image" {
                score += 10_000;
            }
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::core::{GenerateRequest, HealthStatus, ProviderCapabilities};

    struct MockProvider {
        id: &'static str,
        asset_types: &'static [AssetType],
        caps: ProviderCapabilities,
        healthy: bool,
        fail_generate: bool,
    }

    #[async_trait::async_trait]
    impl AssetProvider for MockProvider {
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
    async fn transparent_requests_prefer_gpt_image() {
        let registry = Arc::new(ProviderRegistry::new());

        registry
            .register(Arc::new(MockProvider {
                id: "gemini_image",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    supports_transparency: false,
                    priority: 500,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        registry
            .register(Arc::new(MockProvider {
                id: "gpt_image",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    supports_transparency: true,
                    priority: 1,
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
            params: json!({"transparent": true}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "gpt_image");
    }

    #[tokio::test]
    async fn falls_back_to_next_healthy_provider_when_primary_fails() {
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
                fail_generate: true,
            }))
            .await;

        registry
            .register(Arc::new(MockProvider {
                id: "gpt_image",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    priority: 10,
                    supports_transparency: true,
                    ..Default::default()
                },
                healthy: true,
                fail_generate: false,
            }))
            .await;

        let dispatcher = Dispatcher::new(registry);
        let req = GenerateRequest {
            asset_type: AssetType::Image,
            prompt: Some("landscape".to_string()),
            model: None,
            input_file: None,
            params: json!({"transparent": false}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "gpt_image");
    }

    #[tokio::test]
    async fn skips_unhealthy_provider() {
        let registry = Arc::new(ProviderRegistry::new());

        registry
            .register(Arc::new(MockProvider {
                id: "gemini_image",
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
                id: "gpt_image",
                asset_types: &[AssetType::Image],
                caps: ProviderCapabilities {
                    priority: 5,
                    supports_transparency: true,
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
            params: json!({}),
        };

        let result = dispatcher.dispatch(&req, None).await.unwrap();
        assert_eq!(result.provider_id, "gpt_image");
    }
}
