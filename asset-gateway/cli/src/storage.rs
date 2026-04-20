//! S3-compatible object storage (Bitiful / Aliyun OSS / R2 / MinIO).
//!
//! After a provider returns a `GenerateResponse`, the gateway can
//! optionally upload the asset to an S3 bucket and rewrite `output_url`
//! to a permanent public link.

use base64::Engine;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::sync::Arc;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::core::{AssetType, GenerateResponse};

/// Shared handle to an S3 bucket (or `None` when storage is disabled).
pub type StorageHandle = Option<Arc<ObjectStorage>>;

pub struct ObjectStorage {
    bucket: Box<Bucket>,
    public_url: String,
}

impl ObjectStorage {
    /// Create from app config. Returns `None` if storage is not configured.
    pub fn from_config(config: &AppConfig) -> Option<Arc<Self>> {
        let cfg = config.storage.as_ref()?;
        if cfg.bucket.is_empty() || cfg.access_key.is_empty() {
            return None;
        }

        let region = Region::Custom {
            region: cfg.region.clone().unwrap_or_else(|| "auto".into()),
            endpoint: cfg.endpoint.clone(),
        };

        let credentials = Credentials::new(
            Some(&cfg.access_key),
            Some(&cfg.secret_key),
            None,
            None,
            None,
        )
        .ok()?;

        let bucket = Bucket::new(&cfg.bucket, region, credentials)
            .ok()?
            .with_path_style();

        let public_url = cfg
            .public_url
            .clone()
            .unwrap_or_else(|| format!("{}/{}", cfg.endpoint, cfg.bucket));

        Some(Arc::new(Self { bucket, public_url }))
    }

    /// Upload bytes and return the public URL.
    async fn put(&self, key: &str, data: &[u8], content_type: &str) -> anyhow::Result<String> {
        self.bucket
            .put_object_with_content_type(key, data, content_type)
            .await?;
        let url = format!("{}/{}", self.public_url.trim_end_matches('/'), key);
        Ok(url)
    }

    /// Upload a `GenerateResponse` to OSS, rewriting output fields to a
    /// single permanent `output_url`. Returns the (possibly mutated) response.
    pub async fn upload_response(
        &self,
        mut resp: GenerateResponse,
        asset_type: AssetType,
    ) -> GenerateResponse {
        let ext = asset_type.file_extension();
        let content_type = mime_for_ext(ext);
        let key = format!(
            "assets/{}/{}.{}",
            asset_type.as_str(),
            Uuid::new_v4(),
            ext
        );

        // Determine the raw bytes to upload.
        let bytes = if let Some(ref b64) = resp.output_data {
            // Provider returned base64-encoded data.
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .ok()
        } else if let Some(ref url) = resp.output_url {
            // Provider returned a (possibly temporary) URL — download it.
            download_url(url).await
        } else {
            None
        };

        if let Some(data) = bytes {
            match self.put(&key, &data, content_type).await {
                Ok(url) => {
                    resp.output_url = Some(url);
                    resp.output_data = None; // no need to send base64 anymore
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to upload asset to OSS, returning original response");
                }
            }
        }

        resp
    }
}

/// Best-effort download of a URL into bytes.
async fn download_url(url: &str) -> Option<Vec<u8>> {
    let resp = reqwest::get(url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.bytes().await.ok().map(|b| b.to_vec())
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "glb" => "model/gltf-binary",
        "spz" => "application/octet-stream",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}
