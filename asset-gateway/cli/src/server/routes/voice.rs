use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::providers::qwen_tts::QwenTtsProvider;
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

const DEFAULT_VD_TARGET_MODEL: &str = "qwen3-tts-vd-2026-01-26";
const DEFAULT_LANGUAGE: &str = "zh";

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

async fn get_qwen_provider(state: &Arc<ServerState>) -> AppResult<Arc<dyn crate::core::AssetProvider>> {
    state
        .registry
        .get("qwen_tts")
        .await
        .ok_or_else(|| AppError::not_found("provider not loaded: qwen_tts"))
}

fn parse_voice_type(raw: Option<&str>) -> AppResult<&'static str> {
    match raw
        .unwrap_or("vd")
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

#[derive(Deserialize)]
pub struct SynthesizeReq {
    pub voice: String,
    pub text: String,
    pub model: Option<String>,
    pub language: Option<String>,
}

async fn synthesize_voice(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Json(req): Json<SynthesizeReq>,
) -> AppResult<Json<Value>> {
    let provider = get_qwen_provider(&state).await?;
    let qwen = provider
        .as_any()
        .downcast_ref::<QwenTtsProvider>()
        .ok_or_else(|| AppError::internal("provider qwen_tts has unexpected concrete type"))?;

    let voice = require_non_empty(&req.voice, "voice")?;
    let text = require_non_empty(&req.text, "text")?;

    let data = qwen
        .synthesize(
            voice,
            text,
            req.model.as_deref(),
            req.language.as_deref(),
        )
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "voice.synthesize",
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

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/voice/design", post(design_voice))
        .route("/voice/synthesize", post(synthesize_voice))
        .route("/voice/list", get(list_voices))
        .route("/voice/{voice_id}", delete(delete_voice))
}
