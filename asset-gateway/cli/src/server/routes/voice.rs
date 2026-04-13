use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::core::AssetProvider;
use crate::error::{AppError, AppResult};
use crate::providers::qwen_tts::QwenTtsProvider;
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

const DEFAULT_VC_TARGET_MODEL: &str = "qwen3-tts-vc-2026-01-22";
const DEFAULT_VD_TARGET_MODEL: &str = "qwen3-tts-vd-2026-01-26";
const DEFAULT_VOICE_TYPE: &str = "vc";
const DEFAULT_LANGUAGE: &str = "zh";

#[derive(Deserialize)]
pub struct CloneVoiceReq {
    pub audio_base64: String,
    pub audio_mime: Option<String>,
    pub name: String,
    pub target_model: Option<String>,
}

#[derive(Deserialize)]
pub struct DesignVoiceReq {
    pub voice_prompt: String,
    pub preview_text: String,
    pub name: String,
    pub target_model: Option<String>,
    pub language: Option<String>,
}

#[derive(Deserialize)]
pub struct ListVoicesQuery {
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Deserialize)]
pub struct VoiceTypeQuery {
    #[serde(rename = "type")]
    pub r#type: Option<String>,
}

async fn get_qwen_provider(state: &Arc<ServerState>) -> AppResult<Arc<dyn AssetProvider>> {
    state
        .registry
        .get("qwen_tts")
        .await
        .ok_or_else(|| AppError::not_found("provider not loaded: qwen_tts"))
}

fn parse_voice_type(raw: Option<&str>) -> AppResult<&'static str> {
    match raw
        .unwrap_or(DEFAULT_VOICE_TYPE)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "vc" => Ok("vc"),
        "vd" => Ok("vd"),
        other => Err(AppError::bad_request(format!(
            "invalid voice type '{other}', expected 'vc' or 'vd'"
        ))),
    }
}

fn require_non_empty<'a>(value: &'a str, field: &str) -> AppResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(format!("{field} cannot be empty")));
    }
    Ok(trimmed)
}

async fn clone_voice(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Json(req): Json<CloneVoiceReq>,
) -> AppResult<Json<Value>> {
    let provider = get_qwen_provider(&state).await?;
    let qwen = provider
        .as_any()
        .downcast_ref::<QwenTtsProvider>()
        .ok_or_else(|| AppError::internal("provider qwen_tts has unexpected concrete type"))?;

    let audio_base64 = require_non_empty(&req.audio_base64, "audio_base64")?;
    let name = require_non_empty(&req.name, "name")?;
    let audio_mime = req.audio_mime.as_deref().unwrap_or("audio/mpeg").trim();
    let audio_data = format!("data:{audio_mime};base64,{audio_base64}");
    let target_model = req
        .target_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_VC_TARGET_MODEL);

    let data = qwen
        .clone_voice(target_model, name, &audio_data)
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "voice.clone",
        "data": data,
    })))
}

async fn design_voice(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Json(req): Json<DesignVoiceReq>,
) -> AppResult<Json<Value>> {
    let provider = get_qwen_provider(&state).await?;
    let qwen = provider
        .as_any()
        .downcast_ref::<QwenTtsProvider>()
        .ok_or_else(|| AppError::internal("provider qwen_tts has unexpected concrete type"))?;

    let voice_prompt = require_non_empty(&req.voice_prompt, "voice_prompt")?;
    let preview_text = require_non_empty(&req.preview_text, "preview_text")?;
    let name = require_non_empty(&req.name, "name")?;
    let target_model = req
        .target_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_VD_TARGET_MODEL);
    let language = req
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_LANGUAGE);

    let data = qwen
        .design_voice(target_model, name, voice_prompt, preview_text, language)
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "voice.design",
        "data": data,
    })))
}

async fn list_voices(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Query(query): Query<ListVoicesQuery>,
) -> AppResult<Json<Value>> {
    let provider = get_qwen_provider(&state).await?;
    let qwen = provider
        .as_any()
        .downcast_ref::<QwenTtsProvider>()
        .ok_or_else(|| AppError::internal("provider qwen_tts has unexpected concrete type"))?;

    let voice_type = parse_voice_type(query.r#type.as_deref())?;
    let page = query.page.unwrap_or(0);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let data = qwen
        .list_voices(voice_type, page_size, page)
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "voice.list",
        "data": data,
    })))
}

async fn delete_voice(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Path(voice_id): Path<String>,
    Query(query): Query<VoiceTypeQuery>,
) -> AppResult<Json<Value>> {
    let provider = get_qwen_provider(&state).await?;
    let qwen = provider
        .as_any()
        .downcast_ref::<QwenTtsProvider>()
        .ok_or_else(|| AppError::internal("provider qwen_tts has unexpected concrete type"))?;

    let voice_type = parse_voice_type(query.r#type.as_deref())?;
    let voice_id = require_non_empty(&voice_id, "voice_id")?;

    let data = qwen
        .delete_voice(voice_type, voice_id)
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "voice.delete",
        "data": data,
    })))
}

#[derive(Deserialize)]
pub struct CreateCustomVoiceReq {
    pub voice_prompt: String,
    pub preview_text: String,
    pub name: String,
    pub language: Option<String>,
    pub target_model: Option<String>,
}

async fn create_custom_voice(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Json(req): Json<CreateCustomVoiceReq>,
) -> AppResult<Json<Value>> {
    // Step 1: Get DashScope provider for voice design
    let qwen_provider = get_qwen_provider(&state).await?;
    let qwen = qwen_provider
        .as_any()
        .downcast_ref::<QwenTtsProvider>()
        .ok_or_else(|| AppError::internal("provider qwen_tts has unexpected concrete type"))?;

    let language = req.language.as_deref().unwrap_or(DEFAULT_LANGUAGE);
    let target_model = req
        .target_model
        .as_deref()
        .unwrap_or(DEFAULT_VD_TARGET_MODEL);

    // Step 2: Design voice via DashScope (one-time API call)
    let design_result = qwen
        .design_voice(
            target_model,
            &req.name,
            &req.voice_prompt,
            &req.preview_text,
            language,
        )
        .await
        .map_err(|e| AppError::provider(e.to_string()))?;

    // Step 3: Get VoiceBox provider
    let vb_provider = state
        .registry
        .get("voicebox")
        .await
        .ok_or_else(|| AppError::not_found("provider not loaded: voicebox"))?;
    let vb = vb_provider
        .as_any()
        .downcast_ref::<crate::providers::voicebox::VoiceBoxProvider>()
        .ok_or_else(|| AppError::internal("provider voicebox has unexpected concrete type"))?;

    // Step 4: Ensure VoiceBox model is loaded
    vb.ensure_model_loaded()
        .await
        .map_err(|e| AppError::provider(format!("VoiceBox model load failed: {}", e)))?;

    // Step 5: Create VoiceBox profile
    let profile = vb
        .create_profile(&req.name, language)
        .await
        .map_err(|e| AppError::provider(format!("VoiceBox create profile failed: {}", e)))?;

    let profile_id = profile
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::internal("VoiceBox profile missing id"))?;

    // Step 6: Decode preview audio and upload as sample
    if let Some(preview_audio) = &design_result.preview_audio_data {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let audio_bytes = STANDARD
            .decode(preview_audio)
            .map_err(|e| AppError::internal(format!("Failed to decode preview audio: {}", e)))?;

        vb.upload_sample(profile_id, &audio_bytes, &req.preview_text)
            .await
            .map_err(|e| AppError::provider(format!("VoiceBox upload sample failed: {}", e)))?;
    } else {
        return Err(AppError::provider(
            "DashScope voice design returned no preview audio".to_string(),
        ));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "voice.create_custom",
        "data": {
            "voicebox_profile_id": profile_id,
            "dashscope_voice_id": design_result.voice,
            "name": req.name,
            "language": language,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/voice/clone", post(clone_voice))
        .route("/voice/design", post(design_voice))
        .route("/voice/create-custom", post(create_custom_voice))
        .route("/voice/list", get(list_voices))
        .route("/voice/{voice_id}", delete(delete_voice))
}
