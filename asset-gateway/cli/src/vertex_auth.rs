use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";
const TOKEN_TTL_SECS: i64 = 3600;
const REFRESH_SKEW: Duration = Duration::from_secs(300);

/// Vertex AI 认证 — 通过 Service Account JSON 签发 JWT 换取 OAuth2 access token.
#[derive(Clone)]
pub struct VertexAuth {
    inner: Arc<RwLock<VertexAuthInner>>,
}

struct VertexAuthInner {
    client_email: String,
    private_key: String,
    token: Option<CachedToken>,
    http: reqwest::Client,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct ServiceAccountFile {
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Serialize)]
struct ServiceAccountClaims {
    iss: String,
    scope: &'static str,
    aud: &'static str,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl VertexAuth {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let resolved_path = expand_home(path);
        let content = std::fs::read_to_string(&resolved_path).with_context(|| {
            format!(
                "failed to read Vertex AI service account file: {}",
                resolved_path.display()
            )
        })?;
        let service_account: ServiceAccountFile = serde_json::from_str(&content)
            .context("failed to parse Vertex AI service account JSON")?;

        if service_account.client_email.trim().is_empty() {
            bail!("Vertex AI service account JSON is missing client_email");
        }
        if service_account.private_key.trim().is_empty() {
            bail!("Vertex AI service account JSON is missing private_key");
        }
        if service_account.token_uri.trim().is_empty() {
            bail!("Vertex AI service account JSON is missing token_uri");
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(VertexAuthInner {
                client_email: service_account.client_email,
                private_key: service_account.private_key,
                token: None,
                http: reqwest::Client::new(),
            })),
        })
    }

    pub async fn access_token(&self) -> anyhow::Result<String> {
        {
            let inner = self.inner.read().await;
            if let Some(token) = &inner.token {
                if token_is_fresh(token) {
                    return Ok(token.access_token.clone());
                }
            }
        }

        self.refresh_token().await
    }

    async fn refresh_token(&self) -> anyhow::Result<String> {
        let (client_email, private_key, http) = {
            let inner = self.inner.read().await;
            if let Some(token) = &inner.token {
                if token_is_fresh(token) {
                    return Ok(token.access_token.clone());
                }
            }

            (
                inner.client_email.clone(),
                inner.private_key.clone(),
                inner.http.clone(),
            )
        };

        let now = Utc::now().timestamp();
        let claims = ServiceAccountClaims {
            iss: client_email,
            scope: CLOUD_PLATFORM_SCOPE,
            aud: TOKEN_URI,
            iat: now,
            exp: now + TOKEN_TTL_SECS,
        };
        let encoding_key = EncodingKey::from_rsa_pem(private_key.as_bytes())
            .context("failed to parse Vertex AI service account private key")?;
        let jwt = jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
            .context("failed to sign Vertex AI JWT assertion")?;

        let response = http
            .post(TOKEN_URI)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt.as_str()),
            ])
            .send()
            .await
            .context("failed to exchange Vertex AI JWT for access token")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!(
                "Vertex AI token exchange failed with status {}: {}",
                status,
                body
            );
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("failed to parse Vertex AI token response")?;
        let cached_token = CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        };

        let mut inner = self.inner.write().await;
        if let Some(token) = &inner.token {
            if token_is_fresh(token) {
                return Ok(token.access_token.clone());
            }
        }
        inner.token = Some(cached_token);

        Ok(token_response.access_token)
    }
}

fn token_is_fresh(token: &CachedToken) -> bool {
    token.expires_at > Instant::now() + REFRESH_SKEW
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest);
        }
    }

    PathBuf::from(path)
}
