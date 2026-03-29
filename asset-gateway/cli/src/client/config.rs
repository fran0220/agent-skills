use anyhow::{anyhow, Context};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = "asset-gateway";
const AUTH_FILE_NAME: &str = "auth.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub gateway_url: String,
    pub token: String,
    pub username: String,
    pub expires_at: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AuthStore {
    #[serde(default)]
    records: Vec<AuthConfig>,
}

pub fn config_dir() -> anyhow::Result<PathBuf> {
    let base =
        dirs::config_dir().ok_or_else(|| anyhow!("failed to resolve user config directory"))?;
    Ok(base.join(CONFIG_DIR_NAME))
}

pub fn normalize_gateway_url(gateway_url: &str) -> String {
    let trimmed = gateway_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        gateway_url.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn auth_file_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join(AUTH_FILE_NAME))
}

fn read_store() -> anyhow::Result<AuthStore> {
    let path = auth_file_path()?;
    if !path.exists() {
        return Ok(AuthStore::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read auth config at {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(AuthStore::default());
    }

    serde_json::from_str::<AuthStore>(&raw)
        .with_context(|| format!("failed to parse auth config at {}", path.display()))
}

fn write_store(store: &AuthStore) -> anyhow::Result<()> {
    let path = auth_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(store).context("failed to encode auth store")?;
    fs::write(&path, &content)
        .with_context(|| format!("failed to write auth config at {}", path.display()))?;
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    Ok(())
}

pub fn save_auth(
    gateway_url: &str,
    token: &str,
    expires_at: Option<&str>,
    api_key: Option<&str>,
    username: &str,
) -> anyhow::Result<AuthConfig> {
    let normalized_gateway = normalize_gateway_url(gateway_url);
    let record = AuthConfig {
        gateway_url: normalized_gateway.clone(),
        token: token.to_string(),
        username: username.to_string(),
        expires_at: expires_at.map(str::to_string),
        api_key: api_key.map(str::to_string),
    };

    let mut store = read_store()?;
    if let Some(existing) = store
        .records
        .iter_mut()
        .find(|entry| normalize_gateway_url(&entry.gateway_url) == normalized_gateway)
    {
        *existing = record.clone();
    } else {
        store.records.push(record.clone());
    }

    write_store(&store)?;
    Ok(record)
}

pub fn load_auth(gateway_url: &str) -> anyhow::Result<Option<AuthConfig>> {
    let normalized_gateway = normalize_gateway_url(gateway_url);
    let store = read_store()?;

    Ok(store
        .records
        .into_iter()
        .find(|entry| normalize_gateway_url(&entry.gateway_url) == normalized_gateway))
}

pub fn clear_auth(gateway_url: &str) -> anyhow::Result<bool> {
    let normalized_gateway = normalize_gateway_url(gateway_url);
    let mut store = read_store()?;
    let before = store.records.len();

    store
        .records
        .retain(|entry| normalize_gateway_url(&entry.gateway_url) != normalized_gateway);

    if store.records.len() == before {
        return Ok(false);
    }

    if store.records.is_empty() {
        let path = auth_file_path()?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove auth config at {}", path.display()))?;
        }
        return Ok(true);
    }

    write_store(&store)?;
    Ok(true)
}
