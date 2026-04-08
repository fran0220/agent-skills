use crate::client::{gemini::GeminiClient, llm::LlmClient};
use crate::config::SpriteForgeConfig;
use crate::db::Db;
use crate::types::JobProgress;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppState {
    pub config: SpriteForgeConfig,
    pub gemini: GeminiClient,
    pub llm: LlmClient,
    pub db: Db,
    pub job_channels: Arc<DashMap<String, broadcast::Sender<JobProgress>>>,
}

impl AppState {
    pub fn new(config: SpriteForgeConfig, db: Db) -> Self {
        let gemini = GeminiClient::new(
            &config.gemini_base_url,
            &config.gemini_api_key,
            &config.gemini_model,
        );
        let llm = LlmClient::new(
            &config.llm_proxy_url,
            &config.llm_proxy_key,
            &config.llm_model,
        );
        Self {
            config,
            gemini,
            llm,
            db,
            job_channels: Arc::new(DashMap::new()),
        }
    }

    pub fn job_sender(&self, job_id: &str) -> broadcast::Sender<JobProgress> {
        if let Some(sender) = self.job_channels.get(job_id) {
            return sender.clone();
        }

        let (sender, _) = broadcast::channel(32);
        self.job_channels.insert(job_id.to_string(), sender.clone());
        sender
    }
}
