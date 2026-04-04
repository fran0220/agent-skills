use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{multipart, StatusCode};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::core::*;

const DEFAULT_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MODEL_VERSION: &str = "P1-20260311";
const MAX_POLL_INTERVAL: Duration = Duration::from_secs(20);
const DEFAULT_TRIPO_COST_USD: f64 = 0.20;

#[derive(Clone)]
pub struct TripoClient {
    keys: Vec<String>,
    key_index: Arc<AtomicUsize>,
    base_url: String,
    http: reqwest::Client,
}

impl TripoClient {
    pub fn new(keys: Vec<String>) -> Self {
        assert!(
            !keys.is_empty(),
            "TripoClient requires at least one API key"
        );
        Self {
            keys,
            key_index: Arc::new(AtomicUsize::new(0)),
            base_url: "https://api.tripo3d.ai/v2/openapi".into(),
            http: reqwest::Client::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    fn current_key(&self) -> &str {
        let idx = self.key_index.load(Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    fn next_key(&self) -> &str {
        let idx = self
            .key_index
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1)
            % self.keys.len();
        &self.keys[idx]
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.current_key())
    }

    fn key_count(&self) -> usize {
        self.keys.len()
    }

    pub async fn create_task(&self, body: Value) -> anyhow::Result<String> {
        let max_retries = self.key_count();
        for attempt in 0..max_retries {
            let resp = self
                .http
                .post(format!("{}/task", self.base_url))
                .header("Authorization", self.auth_header())
                .json(&body)
                .send()
                .await?;

            let status = resp.status();
            let text = resp.text().await?;

            if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::PAYMENT_REQUIRED {
                if attempt + 1 < max_retries {
                    let key_hint = &self.current_key()[..8.min(self.current_key().len())];
                    tracing::warn!(
                        status = %status,
                        key = key_hint,
                        attempt = attempt + 1,
                        "Tripo3D key limited, rotating to next key"
                    );
                    self.next_key();
                    continue;
                }
            }

            if !status.is_success() {
                anyhow::bail!("Tripo3D task creation returned {}: {}", status, text);
            }

            let payload: Value = serde_json::from_str(&text)?;
            return payload["data"]["task_id"]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("missing task_id in Tripo3D response"));
        }
        anyhow::bail!(
            "Tripo3D: all {} keys exhausted (rate-limited or insufficient balance)",
            max_retries
        )
    }

    pub async fn poll_task(&self, task_id: &str, timeout_secs: u64) -> anyhow::Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        let mut poll_interval = Duration::from_secs(2);

        loop {
            if Instant::now() >= deadline {
                anyhow::bail!("Tripo3D task {} timed out after {}s", task_id, timeout_secs);
            }

            tokio::time::sleep(poll_interval).await;

            let resp = self
                .http
                .get(format!("{}/task/{}", self.base_url, task_id))
                .header("Authorization", self.auth_header())
                .send()
                .await?;

            let status = resp.status();
            let text = resp.text().await?;

            if !status.is_success() {
                if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                    poll_interval = (poll_interval * 2).min(MAX_POLL_INTERVAL);
                    continue;
                }

                anyhow::bail!(
                    "Tripo3D polling returned {} for task {}: {}",
                    status,
                    task_id,
                    text
                );
            }

            let payload: Value = serde_json::from_str(&text)?;
            let task_status = payload["data"]["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_ascii_lowercase();

            match task_status.as_str() {
                "success" | "completed" | "succeeded" => return Ok(payload),
                "failed" | "error" | "canceled" | "cancelled" => {
                    anyhow::bail!(
                        "Tripo3D task {} failed: {}",
                        task_id,
                        extract_error_message(&payload)
                    );
                }
                _ => {
                    poll_interval = (poll_interval * 2).min(MAX_POLL_INTERVAL);
                }
            }
        }
    }

    pub async fn upload_file(&self, file_bytes: &[u8], file_type: &str) -> anyhow::Result<String> {
        let max_retries = self.key_count();
        for attempt in 0..max_retries {
            for endpoint in ["upload/sts", "upload"] {
                let part = multipart::Part::bytes(file_bytes.to_vec())
                    .file_name(format!("upload.{}", normalize_file_type(file_type)))
                    .mime_str(mime_for_file_type(file_type))?;
                let form = multipart::Form::new().part("file", part);

                let resp = self
                    .http
                    .post(format!("{}/{}", self.base_url, endpoint))
                    .header("Authorization", self.auth_header())
                    .multipart(form)
                    .send()
                    .await?;

                let status = resp.status();
                let text = resp.text().await?;

                if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
                    continue;
                }

                if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::PAYMENT_REQUIRED
                {
                    if attempt + 1 < max_retries {
                        let key_hint = &self.current_key()[..8.min(self.current_key().len())];
                        tracing::warn!(
                            status = %status,
                            key = key_hint,
                            attempt = attempt + 1,
                            "Tripo3D upload key limited, rotating to next key"
                        );
                        self.next_key();
                        break;
                    }
                }

                if !status.is_success() {
                    anyhow::bail!("Tripo3D file upload returned {}: {}", status, text);
                }

                let payload: Value = serde_json::from_str(&text)?;
                if let Some(token) = payload["data"]["image_token"]
                    .as_str()
                    .or_else(|| payload["data"]["file_token"].as_str())
                {
                    return Ok(token.to_string());
                }

                anyhow::bail!("Tripo3D upload response did not include an image token");
            }
        }

        anyhow::bail!("Tripo3D upload endpoint not found")
    }

    pub async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/user/balance", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await;

        let key_info = format!("{} key(s) configured", self.key_count());
        match resp {
            Ok(r) => Ok(HealthStatus {
                healthy: r.status().is_success(),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(key_info),
            }),
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(format!("{} ({})", e, key_info)),
            }),
        }
    }

    pub async fn query_all_balances(&self) -> Vec<Value> {
        let mut results = Vec::with_capacity(self.keys.len());
        for (i, key) in self.keys.iter().enumerate() {
            let masked = if key.len() > 12 {
                format!("{}...{}", &key[..8], &key[key.len() - 4..])
            } else {
                "***".to_string()
            };

            match self
                .http
                .get(format!("{}/user/balance", self.base_url))
                .header("Authorization", format!("Bearer {}", key))
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if let Ok(text) = resp.text().await {
                        if let Ok(payload) = serde_json::from_str::<Value>(&text) {
                            let balance = payload["data"]["balance"].as_f64().unwrap_or(0.0);
                            let frozen = payload["data"]["frozen"].as_f64().unwrap_or(0.0);
                            results.push(json!({
                                "index": i,
                                "key_masked": masked,
                                "balance": balance,
                                "frozen": frozen,
                                "available": balance - frozen,
                                "status": if status.is_success() { "ok" } else { "error" },
                            }));
                            continue;
                        }
                    }
                    results.push(json!({
                        "index": i,
                        "key_masked": masked,
                        "balance": 0,
                        "frozen": 0,
                        "available": 0,
                        "status": "error",
                        "error": format!("HTTP {}", status),
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "index": i,
                        "key_masked": masked,
                        "balance": 0,
                        "frozen": 0,
                        "available": 0,
                        "status": "error",
                        "error": e.to_string(),
                    }));
                }
            }
        }
        results
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Process3dResponse {
    pub output_url: Option<String>,
    pub metadata: Value,
    pub elapsed_ms: u64,
}

/// Tripo3D provider — image/text to 3D model generation and post-processing.
pub struct Tripo3dProvider {
    pub id: String,
    client: TripoClient,
}

impl Tripo3dProvider {
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            id: "tripo3d".into(),
            client: TripoClient::new(keys),
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.client = self.client.with_base_url(url);
        self
    }

    pub fn client(&self) -> &TripoClient {
        &self.client
    }

    const RESERVED_GENERATE_PARAMS: &[&str] = &[
        "timeout_seconds",
        "quality",
        "transparent",
        "stream",
        "output_format",
        "multiview",
    ];

    async fn build_task_body(&self, req: &GenerateRequest) -> anyhow::Result<Value> {
        let mut body = if let Some(multiview) = req.params.get("multiview") {
            json!({
                "type": "multiview_to_model",
                "files": self.build_multiview_files(multiview).await?,
            })
        } else if let Some(input_file) = req.input_file.as_ref() {
            json!({
                "type": "image_to_model",
                "file": self.build_file_input(input_file).await?,
            })
        } else if let Some(prompt) = req.prompt.as_ref() {
            json!({
                "type": "text_to_model",
                "prompt": prompt,
            })
        } else {
            anyhow::bail!("Tripo3D requires either input_file, multiview, or prompt");
        };

        let params = req.params.as_object();
        body["model_version"] = json!(self.model_version(req));
        body["pbr"] = json!(bool_param(params, "pbr").unwrap_or(true));
        body["texture"] = json!(bool_param(params, "texture").unwrap_or(true));

        if let Some(face_limit) = u64_param(params, "face_limit") {
            validate_face_limit(face_limit)?;
            body["face_limit"] = json!(face_limit);
        }

        copy_string_param(&mut body, params, "texture_quality")?;
        copy_bool_param(&mut body, params, "auto_size")?;
        copy_string_param(&mut body, params, "negative_prompt")?;
        copy_string_param(&mut body, params, "style")?;

        if let Some(params_obj) = params {
            let body_obj = body
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("Tripo3D request body must be a JSON object"))?;

            for (key, value) in params_obj {
                if Self::RESERVED_GENERATE_PARAMS.contains(&key.as_str())
                    || body_obj.contains_key(key)
                {
                    continue;
                }
                body_obj.insert(key.clone(), value.clone());
            }
        }

        Ok(body)
    }

    async fn build_multiview_files(&self, multiview: &Value) -> anyhow::Result<Vec<Value>> {
        let Some(entries) = multiview.as_array() else {
            anyhow::bail!("multiview must be an array of 4 image inputs");
        };

        if entries.len() != 4 {
            anyhow::bail!("multiview must contain exactly 4 image inputs: front,left,back,right");
        }

        let mut files = Vec::with_capacity(4);
        for entry in entries {
            let Some(input) = entry.as_str() else {
                anyhow::bail!("multiview entries must be strings");
            };
            files.push(self.build_file_input(input).await?);
        }
        Ok(files)
    }

    async fn build_file_input(&self, input: &str) -> anyhow::Result<Value> {
        if input.starts_with("http://") || input.starts_with("https://") {
            let file_type = infer_file_type(input, None);
            return Ok(json!({
                "type": normalize_file_type(&file_type),
                "url": input,
            }));
        }

        let (bytes, file_type) = if input.starts_with("data:") {
            decode_data_uri(input)?
        } else if Path::new(input).exists() {
            let bytes = tokio::fs::read(input).await?;
            let file_type = infer_file_type(input, None);
            (bytes, file_type)
        } else {
            anyhow::bail!(
                "unsupported Tripo3D file input: expected URL, data URI, or local file path"
            );
        };

        let token = self.client.upload_file(&bytes, &file_type).await?;
        Ok(json!({
            "type": normalize_file_type(&file_type),
            "file_token": token,
        }))
    }

    fn model_version(&self, req: &GenerateRequest) -> String {
        req.params
            .get("model_version")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| req.model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL_VERSION.to_string())
    }

    fn output_metadata(task_id: &str, payload: &Value, operation: Option<&str>) -> Value {
        let output = payload["data"]["output"].clone();
        let rendered_image = output.get("rendered_image").cloned().unwrap_or(Value::Null);
        let mut metadata = json!({
            "tripo_task_id": task_id,
            "task_id": task_id,
            "status": payload["data"]["status"].clone(),
            "rendered_image": rendered_image,
            "output": output,
        });

        if let Some(operation) = operation {
            metadata["operation"] = json!(operation);
        }

        metadata
    }

    /// Extract the model download URL from the completed task response.
    /// Tripo3D response: `data.output.model` or `data.output.pbr_model` (URL strings).
    pub fn extract_model_url(payload: &Value) -> Option<String> {
        let output = &payload["data"]["output"];
        output["model"]
            .as_str()
            .or_else(|| output["pbr_model"].as_str())
            .or_else(|| output["base_model"].as_str())
            .map(str::to_string)
    }

    async fn build_process_task_body(
        &self,
        tripo_task_id: &str,
        operation: &str,
        params: &Value,
    ) -> anyhow::Result<Value> {
        let params = params
            .as_object()
            .cloned()
            .unwrap_or_else(Map::<String, Value>::new);

        let mut body = json!({
            "original_model_task_id": tripo_task_id,
        });

        match operation {
            "convert" => {
                body["type"] = json!("convert_model");
                body["format"] = json!(required_string(&params, "format")?);
                copy_allowed_fields(
                    &mut body,
                    &params,
                    &[
                        "quad",
                        "face_limit",
                        "force_symmetry",
                        "flatten_bottom",
                        "flatten_bottom_threshold",
                        "texture_size",
                        "texture_format",
                        "pack_uv",
                        "bake",
                        "pivot_to_center_bottom",
                        "scale_factor",
                        "part_names",
                        "with_animation",
                        "animate_in_place",
                        "export_orientation",
                        "export_vertex_colors",
                        "compress",
                        "fbx_preset",
                    ],
                )?;
            }
            "texture" => {
                body["type"] = json!("texture_model");
                if let Some(texture_prompt) = params.get("texture_prompt") {
                    body["texture_prompt"] = texture_prompt.clone();
                } else if let Some(prompt) = optional_string(&params, "prompt") {
                    body["texture_prompt"] = json!({ "text": prompt });
                }
                copy_allowed_fields(
                    &mut body,
                    &params,
                    &[
                        "pbr",
                        "texture_quality",
                        "texture_seed",
                        "texture_alignment",
                        "bake",
                        "compress",
                        "part_names",
                        "model_version",
                        "texture",
                    ],
                )?;

                if let Some(style_image) = optional_string(&params, "style_image") {
                    let style_image = self.build_file_input(&style_image).await?;
                    ensure_object_field(&mut body, "texture_prompt")?
                        .insert("style_image".into(), style_image);
                }

                if let Some(image) = optional_string(&params, "image") {
                    let image = self.build_file_input(&image).await?;
                    ensure_object_field(&mut body, "texture_prompt")?.insert("image".into(), image);
                }

                if let Some(images) = params.get("images") {
                    let images = self.build_multiview_files(images).await?;
                    ensure_object_field(&mut body, "texture_prompt")?
                        .insert("images".into(), Value::Array(images));
                }
            }
            "rig" => {
                body["type"] = json!("animate_rig");
                copy_allowed_fields(
                    &mut body,
                    &params,
                    &["out_format", "spec", "rig_type", "model_version"],
                )?;
            }
            "animate" => {
                body["type"] = json!("animate_retarget");
                copy_allowed_fields(
                    &mut body,
                    &params,
                    &[
                        "animation",
                        "animations",
                        "out_format",
                        "bake_animation",
                        "animate_in_place",
                        "export_with_geometry",
                    ],
                )?;
                if body.get("animation").is_none() && body.get("animations").is_none() {
                    anyhow::bail!("animate requires animation or animations");
                }
            }
            "reduce" => {
                body["type"] = json!("highpoly_to_lowpoly");
                copy_allowed_fields(
                    &mut body,
                    &params,
                    &["quad", "face_limit", "force_symmetry"],
                )?;
            }
            "stylize" => {
                body["type"] = json!("stylize_model");
                body["style"] = json!(required_string(&params, "style")?);
                copy_allowed_fields(&mut body, &params, &["block_size"])?;
            }
            "segment" => {
                body["type"] = json!("mesh_segmentation");
            }
            "prerigcheck" => {
                body["type"] = json!("animate_prerigcheck");
            }
            "refine" => {
                body["type"] = json!("refine_model");
                body.as_object_mut()
                    .unwrap()
                    .remove("original_model_task_id");
                body["draft_model_task_id"] = json!(tripo_task_id);
            }
            "import" => {
                body["type"] = json!("import_model");
                body.as_object_mut()
                    .unwrap()
                    .remove("original_model_task_id");
                let file_input = if let Some(url) = optional_string(&params, "file_url") {
                    self.build_file_input(&url).await?
                } else if let Some(path) = optional_string(&params, "file_path") {
                    self.build_file_input(&path).await?
                } else {
                    anyhow::bail!("import requires file_url or file_path");
                };
                body["file"] = file_input;
            }
            _ => anyhow::bail!("unsupported Tripo3D operation: {}", operation),
        }

        if let Some(face_limit) = body.get("face_limit").and_then(Value::as_u64) {
            validate_face_limit(face_limit)?;
        }

        Ok(body)
    }

    pub async fn process3d(
        &self,
        tripo_task_id: &str,
        operation: &str,
        params: &Value,
    ) -> anyhow::Result<Process3dResponse> {
        let start = Instant::now();
        let body = self
            .build_process_task_body(tripo_task_id, operation, params)
            .await?;
        let timeout_secs = params
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let task_id = self.client.create_task(body).await?;
        let payload = self.client.poll_task(&task_id, timeout_secs).await?;

        Ok(Process3dResponse {
            output_url: Self::extract_model_url(&payload),
            metadata: Self::output_metadata(&task_id, &payload, Some(operation)),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[async_trait::async_trait]
impl AssetProvider for Tripo3dProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Tripo3D (Image/Text to 3D)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Model3d]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 2,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let start = Instant::now();
        let body = self.build_task_body(req).await?;
        let timeout_secs = req
            .params
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let task_id = self.client.create_task(body).await?;
        let payload = self.client.poll_task(&task_id, timeout_secs).await?;
        let model_url = Self::extract_model_url(&payload).ok_or_else(|| {
            anyhow::anyhow!(
                "Tripo3D task completed without model URL. output: {}",
                serde_json::to_string(&payload["data"]["output"]).unwrap_or_default()
            )
        })?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(model_url),
            output_data: None,
            metadata: Self::output_metadata(&task_id, &payload, None),
            cost_usd: Some(DEFAULT_TRIPO_COST_USD),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        self.client.health_check().await
    }
}

fn bool_param(params: Option<&Map<String, Value>>, key: &str) -> Option<bool> {
    params
        .and_then(|values| values.get(key))
        .and_then(Value::as_bool)
}

fn u64_param(params: Option<&Map<String, Value>>, key: &str) -> Option<u64> {
    params
        .and_then(|values| values.get(key))
        .and_then(Value::as_u64)
}

fn copy_string_param(
    body: &mut Value,
    params: Option<&Map<String, Value>>,
    key: &str,
) -> anyhow::Result<()> {
    if let Some(value) = params.and_then(|values| values.get(key)) {
        if value.is_null() {
            return Ok(());
        }
        let Some(text) = value.as_str() else {
            anyhow::bail!("{} must be a string", key);
        };
        body[key] = json!(text);
    }
    Ok(())
}

fn copy_bool_param(
    body: &mut Value,
    params: Option<&Map<String, Value>>,
    key: &str,
) -> anyhow::Result<()> {
    if let Some(value) = params.and_then(|values| values.get(key)) {
        if value.is_null() {
            return Ok(());
        }
        let Some(flag) = value.as_bool() else {
            anyhow::bail!("{} must be a boolean", key);
        };
        body[key] = json!(flag);
    }
    Ok(())
}

fn copy_allowed_fields(
    body: &mut Value,
    params: &Map<String, Value>,
    keys: &[&str],
) -> anyhow::Result<()> {
    let body_obj = body
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Tripo3D request body must be a JSON object"))?;

    for key in keys {
        if let Some(value) = params.get(*key) {
            body_obj.insert((*key).to_string(), value.clone());
        }
    }

    Ok(())
}

fn ensure_object_field<'a>(
    body: &'a mut Value,
    key: &str,
) -> anyhow::Result<&'a mut Map<String, Value>> {
    if body.get(key).is_none() {
        body[key] = json!({});
    }

    body.get_mut(key)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow::anyhow!("{} must be a JSON object", key))
}

fn required_string(params: &Map<String, Value>, key: &str) -> anyhow::Result<String> {
    optional_string(params, key).ok_or_else(|| anyhow::anyhow!("{} is required", key))
}

fn optional_string(params: &Map<String, Value>, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn validate_face_limit(face_limit: u64) -> anyhow::Result<()> {
    if !(48..=20_000).contains(&face_limit) {
        anyhow::bail!("face_limit must be between 48 and 20000");
    }
    Ok(())
}

fn extract_error_message(payload: &Value) -> String {
    let error = &payload["data"]["error"];
    if let Some(message) = error.as_str() {
        return message.to_string();
    }
    if !error.is_null() {
        return serde_json::to_string(error).unwrap_or_else(|_| "unknown error".to_string());
    }
    "unknown error".to_string()
}

fn decode_data_uri(input: &str) -> anyhow::Result<(Vec<u8>, String)> {
    let (meta, encoded) = input
        .split_once(',')
        .ok_or_else(|| anyhow::anyhow!("invalid data URI"))?;
    let mime = meta
        .strip_prefix("data:")
        .and_then(|value| value.split(';').next())
        .filter(|value| !value.is_empty())
        .unwrap_or("image/png");
    let bytes = STANDARD.decode(encoded)?;
    Ok((bytes, infer_file_type("", Some(mime))))
}

fn infer_file_type(input: &str, mime_hint: Option<&str>) -> String {
    if let Some(mime) = mime_hint {
        if mime.contains("webp") {
            return "webp".into();
        }
        if mime.contains("png") {
            return "png".into();
        }
        if mime.contains("jpeg") || mime.contains("jpg") {
            return "jpg".into();
        }
    }

    Path::new(input)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .map(|ext| match ext.as_str() {
            "jpeg" | "jpg" => "jpg".to_string(),
            "png" => "png".to_string(),
            "webp" => "webp".to_string(),
            _ => "jpg".to_string(),
        })
        .unwrap_or_else(|| "jpg".to_string())
}

fn normalize_file_type(file_type: &str) -> &'static str {
    if file_type.eq_ignore_ascii_case("png") {
        "png"
    } else if file_type.eq_ignore_ascii_case("webp") {
        "webp"
    } else {
        "jpg"
    }
}

fn mime_for_file_type(file_type: &str) -> &'static str {
    match normalize_file_type(file_type) {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn build_task_body_uses_p1_defaults_and_params() {
        let provider = Tripo3dProvider::new(vec!["test-key".into()]);
        let req = GenerateRequest {
            asset_type: AssetType::Model3d,
            prompt: Some("a toy robot".into()),
            model: None,
            input_file: None,
            reference_images: vec![],
            edit_mode: None,
            session_id: None,
            params: json!({
                "face_limit": 4096,
                "texture_quality": "detailed",
                "auto_size": true,
                "negative_prompt": "blurry",
                "style": "gold"
            }),
        };

        let body = provider.build_task_body(&req).await.unwrap();

        assert_eq!(body["type"], "text_to_model");
        assert_eq!(body["model_version"], DEFAULT_MODEL_VERSION);
        assert_eq!(body["face_limit"], 4096);
        assert_eq!(body["pbr"], true);
        assert_eq!(body["texture"], true);
        assert_eq!(body["texture_quality"], "detailed");
        assert_eq!(body["auto_size"], true);
        assert_eq!(body["negative_prompt"], "blurry");
        assert_eq!(body["style"], "gold");
    }

    #[tokio::test]
    async fn build_task_body_uses_multiview_inputs() {
        let provider = Tripo3dProvider::new(vec!["test-key".into()]);
        let req = GenerateRequest {
            asset_type: AssetType::Model3d,
            prompt: None,
            model: None,
            input_file: None,
            reference_images: vec![],
            edit_mode: None,
            session_id: None,
            params: json!({
                "multiview": [
                    "https://example.com/front.jpg",
                    "https://example.com/left.jpg",
                    "https://example.com/back.jpg",
                    "https://example.com/right.jpg"
                ]
            }),
        };

        let body = provider.build_task_body(&req).await.unwrap();

        assert_eq!(body["type"], "multiview_to_model");
        assert_eq!(body["files"].as_array().unwrap().len(), 4);
        assert_eq!(body["files"][0]["url"], "https://example.com/front.jpg");
        assert_eq!(body["files"][3]["url"], "https://example.com/right.jpg");
    }

    #[tokio::test]
    async fn build_process_task_body_wraps_texture_prompt() {
        let provider = Tripo3dProvider::new(vec!["test-key".into()]);
        let body = provider
            .build_process_task_body(
                "task-123",
                "texture",
                &json!({
                    "prompt": "weathered bronze",
                    "pbr": true,
                    "texture_quality": "detailed"
                }),
            )
            .await
            .unwrap();

        assert_eq!(body["type"], "texture_model");
        assert_eq!(body["original_model_task_id"], "task-123");
        assert_eq!(body["texture_prompt"]["text"], "weathered bronze");
        assert_eq!(body["pbr"], true);
        assert_eq!(body["texture_quality"], "detailed");
    }

    #[tokio::test]
    async fn build_process_task_body_texture_supports_extended_fields_and_images() {
        let provider = Tripo3dProvider::new(vec!["test-key".into()]);
        let body = provider
            .build_process_task_body(
                "task-123",
                "texture",
                &json!({
                    "texture_prompt": {
                        "text": "weathered bronze"
                    },
                    "texture_alignment": "geometry",
                    "bake": true,
                    "compress": true,
                    "part_names": ["body", "handle"],
                    "model_version": "texture-v2",
                    "texture": false,
                    "style_image": "https://example.com/style.png",
                    "image": "https://example.com/reference.jpg",
                    "images": [
                        "https://example.com/front.jpg",
                        "https://example.com/left.jpg",
                        "https://example.com/back.jpg",
                        "https://example.com/right.jpg"
                    ]
                }),
            )
            .await
            .unwrap();

        assert_eq!(body["texture_alignment"], "geometry");
        assert_eq!(body["bake"], true);
        assert_eq!(body["compress"], true);
        assert_eq!(body["part_names"], json!(["body", "handle"]));
        assert_eq!(body["model_version"], "texture-v2");
        assert_eq!(body["texture"], false);
        assert_eq!(body["texture_prompt"]["text"], "weathered bronze");
        assert_eq!(
            body["texture_prompt"]["style_image"]["url"],
            "https://example.com/style.png"
        );
        assert_eq!(
            body["texture_prompt"]["image"]["url"],
            "https://example.com/reference.jpg"
        );
        assert_eq!(
            body["texture_prompt"]["images"].as_array().unwrap().len(),
            4
        );
        assert_eq!(
            body["texture_prompt"]["images"][2]["url"],
            "https://example.com/back.jpg"
        );
    }
}
