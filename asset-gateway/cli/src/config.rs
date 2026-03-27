use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub vault_key: String,
    pub data_dir: String,
}

impl AppConfig {
    pub fn load(host: String, port: u16, database_url: String) -> anyhow::Result<Self> {
        let database_url = if database_url.trim().is_empty() {
            std::env::var("ASSET_GATEWAY_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/asset_gateway".into())
        } else {
            database_url
        };

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret: std::env::var("ASSET_GATEWAY_JWT_SECRET")
                .unwrap_or_else(|_| "asset-gw-dev-secret-change-me".into()),
            vault_key: std::env::var("ASSET_GATEWAY_VAULT_KEY")
                .unwrap_or_else(|_| "asset-gw-dev-vault-key-32ch!".into()),
            data_dir: std::env::var("ASSET_GATEWAY_DATA_DIR").unwrap_or_else(|_| "./data".into()),
        })
    }
}
