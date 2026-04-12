//! Smoke test for Vertex AI integration.
//!
//! Run: cargo test --test vertex_smoke -- --nocapture

use asset_gateway::vertex_auth::VertexAuth;

const SA_PATH: &str = "~/vertex-ai-key.json";
const PROJECT: &str = "gemini-workspace-2026";
const LOCATION: &str = "global";

fn expand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

fn vertex_url(model: &str) -> String {
    format!(
        "https://aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
        PROJECT, LOCATION, model
    )
}

#[tokio::test]
async fn vertex_auth_token_exchange() {
    let path = expand(SA_PATH);
    if !std::path::Path::new(&path).exists() {
        eprintln!("SKIP");
        return;
    }

    let auth = VertexAuth::from_file(&path).expect("load SA");
    let t1 = auth.access_token().await.expect("token");
    assert!(t1.len() > 100);
    let t2 = auth.access_token().await.expect("cached");
    assert_eq!(t1, t2);
    eprintln!("✅ Auth + cache OK");
}

#[tokio::test]
async fn vertex_gemini_image() {
    let path = expand(SA_PATH);
    if !std::path::Path::new(&path).exists() {
        eprintln!("SKIP");
        return;
    }

    let auth = VertexAuth::from_file(&path).expect("load SA");
    let token = auth.access_token().await.expect("token");
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "contents": [{"role": "user", "parts": [{"text": "A blue star icon"}]}],
        "generationConfig": {"responseModalities": ["IMAGE", "TEXT"]}
    });

    let resp = client
        .post(vertex_url("gemini-3.1-flash-image-preview"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .expect("send");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(
        status.is_success(),
        "gemini-3.1-flash-image-preview failed {}: {}",
        status,
        &text[..text.len().min(300)]
    );

    let payload: serde_json::Value = serde_json::from_str(&text).unwrap();
    let parts = payload["candidates"][0]["content"]["parts"]
        .as_array()
        .expect("parts");
    let has_image = parts.iter().any(|p| p["inlineData"]["data"].is_string());
    assert!(has_image);
    eprintln!(
        "✅ gemini-3.1-flash-image-preview OK ({} parts)",
        parts.len()
    );
}

#[tokio::test]
async fn vertex_lyria() {
    let path = expand(SA_PATH);
    if !std::path::Path::new(&path).exists() {
        eprintln!("SKIP");
        return;
    }

    let auth = VertexAuth::from_file(&path).expect("load SA");
    let token = auth.access_token().await.expect("token");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let body = serde_json::json!({
        "contents": [{"role": "user", "parts": [{"text": "Happy upbeat piano"}]}],
        "generationConfig": {"responseModalities": ["AUDIO", "TEXT"]}
    });

    let resp = client
        .post(vertex_url("lyria-3-clip-preview"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .expect("send");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(
        status.is_success(),
        "lyria-3-clip-preview failed {}: {}",
        status,
        &text[..text.len().min(300)]
    );

    let payload: serde_json::Value = serde_json::from_str(&text).unwrap();
    let parts = payload["candidates"][0]["content"]["parts"]
        .as_array()
        .expect("parts");
    let has_audio = parts.iter().any(|p| p["inlineData"]["data"].is_string());
    assert!(has_audio);
    eprintln!("✅ lyria-3-clip-preview OK");
}
