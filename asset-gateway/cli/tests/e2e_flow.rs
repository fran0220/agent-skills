use std::env;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, Row};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use uuid::Uuid;

struct AbortOnDrop {
    handle: Option<JoinHandle<anyhow::Result<()>>>,
}

impl AbortOnDrop {
    fn new(handle: JoinHandle<anyhow::Result<()>>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

fn replace_database_name(url: &str, database_name: &str) -> String {
    let (base, query) = match url.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (url, None),
    };

    let authority_start = base.find("://").map(|idx| idx + 3).unwrap_or(0);
    let path_start = base[authority_start..]
        .find('/')
        .map(|idx| authority_start + idx);

    let rewritten = match path_start {
        Some(idx) => format!("{}/{}", &base[..idx], database_name),
        None => format!("{}/{}", base, database_name),
    };

    match query {
        Some(query) => format!("{}?{}", rewritten, query),
        None => rewritten,
    }
}

fn database_name_from_url(url: &str) -> Result<String> {
    let base = url
        .split_once('?')
        .map_or(url, |(without_query, _)| without_query);
    let database = base
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("cannot parse database name from url: {}", url))?;
    Ok(database.to_string())
}

fn random_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

async fn ensure_database_exists(admin_database_url: &str, database_name: &str) -> Result<()> {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin_database_url)
        .await
        .with_context(|| {
            format!(
                "failed to connect admin database for setup: {}",
                admin_database_url
            )
        })?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(database_name)
        .fetch_optional(&admin_pool)
        .await?;

    if exists.is_none() {
        let escaped_name = database_name.replace('"', "\"\"");
        let create_sql = format!("CREATE DATABASE \"{}\"", escaped_name);
        sqlx::query(&create_sql).execute(&admin_pool).await?;
    }

    Ok(())
}

async fn reset_tables(database_url: &str) -> Result<()> {
    let pool = asset_gateway::db::connect(database_url).await?;

    for stmt in [
        "DELETE FROM jobs",
        "DELETE FROM providers",
        "DELETE FROM credentials",
        "DELETE FROM users",
    ] {
        sqlx::query(stmt).execute(&pool).await?;
    }

    Ok(())
}

async fn mock_models() -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [
            { "id": "mock-model" }
        ]
    }))
}

async fn mock_openai_chat(Json(body): Json<Value>) -> Json<Value> {
    let prompt = body
        .pointer("/messages/0/content")
        .and_then(Value::as_str)
        .unwrap_or_default();

    Json(json!({
        "id": "chatcmpl-mock",
        "choices": [
            {
                "message": {
                    "content": format!("mocked: {}", prompt)
                }
            }
        ]
    }))
}

async fn start_mock_provider_server() -> Result<(String, JoinHandle<anyhow::Result<()>>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new()
        .route("/v1/models", get(mock_models))
        .route("/v1/chat/completions", post(mock_openai_chat));

    let task = tokio::spawn(async move {
        axum::serve(listener, app).await?;
        Ok(())
    });

    Ok((format!("http://{}", addr), task))
}

async fn wait_for_gateway_ready(client: &reqwest::Client, gateway_url: &str) -> Result<()> {
    let health_url = format!("{}/api/health", gateway_url);

    for _ in 0..80 {
        if let Ok(response) = client.get(&health_url).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow!(
        "gateway did not become healthy in time: {}",
        health_url
    ))
}

#[tokio::test]
async fn e2e_server_flow_covers_required_scenarios() -> Result<()> {
    let test_db_url = env::var("ASSET_GATEWAY_TEST_DATABASE_URL")
        .or_else(|_| env::var("ASSET_GATEWAY_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://localhost/asset_gateway_e2e".to_string());
    let admin_db_url = env::var("ASSET_GATEWAY_TEST_ADMIN_DATABASE_URL")
        .unwrap_or_else(|_| replace_database_name(&test_db_url, "postgres"));

    let db_name = database_name_from_url(&test_db_url)?;
    ensure_database_exists(&admin_db_url, &db_name).await?;
    reset_tables(&test_db_url).await?;

    let (mock_provider_url, mock_provider_task) = start_mock_provider_server().await?;
    let _mock_provider_guard = AbortOnDrop::new(mock_provider_task);

    let gateway_port = random_port()?;
    let gateway_url = format!("http://127.0.0.1:{}", gateway_port);
    let server_task = tokio::spawn({
        let database_url = test_db_url.clone();
        async move { asset_gateway::server::run("127.0.0.1".into(), gateway_port, database_url).await }
    });
    let _server_guard = AbortOnDrop::new(server_task);

    let client = reqwest::Client::new();
    wait_for_gateway_ready(&client, &gateway_url).await?;

    let suffix = Uuid::new_v4().simple().to_string();
    let admin_username = format!("admin_{}", suffix);
    let admin_password = format!("Pass-{}-123456", suffix);
    let normal_username = format!("user_{}", suffix);
    let normal_password = format!("User-{}-123456", suffix);
    let provider_id = format!("mock-llm-{}", suffix);
    let credential_secret = format!("secret-{}-token", suffix);

    // 1) 注册 + 登录获取 token
    let register_admin = client
        .post(format!("{}/auth/register", gateway_url))
        .json(&json!({
            "username": &admin_username,
            "password": &admin_password,
        }))
        .send()
        .await?;
    assert!(register_admin.status().is_success());
    let register_admin_body: Value = register_admin.json().await?;
    assert_eq!(register_admin_body["ok"], json!(true));
    assert_eq!(register_admin_body["data"]["role"], json!("admin"));

    let login_admin = client
        .post(format!("{}/auth/login", gateway_url))
        .json(&json!({
            "username": &admin_username,
            "password": &admin_password,
        }))
        .send()
        .await?;
    assert!(login_admin.status().is_success());
    let login_admin_body: Value = login_admin.json().await?;
    let admin_token = login_admin_body["data"]["token"]
        .as_str()
        .context("missing admin token")?
        .to_string();

    // 2) 设置凭据（验证加密存储）
    let set_credential = client
        .put(format!("{}/api/credentials", gateway_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({
            "key": "api_key",
            "value": &credential_secret,
            "provider_id": &provider_id,
            "description": "e2e credential",
        }))
        .send()
        .await?;
    assert_eq!(set_credential.status(), reqwest::StatusCode::OK);
    let set_credential_body: Value = set_credential.json().await?;
    assert_eq!(set_credential_body["ok"], json!(true));
    assert_eq!(set_credential_body["command"], json!("credential.set"));

    let verify_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url)
        .await?;
    let credential_row = sqlx::query(
        "SELECT encrypted_value, nonce FROM credentials WHERE key = $1 AND provider_id = $2",
    )
    .bind("api_key")
    .bind(&provider_id)
    .fetch_one(&verify_pool)
    .await?;
    let encrypted_value: Vec<u8> = credential_row.try_get("encrypted_value")?;
    let nonce: Vec<u8> = credential_row.try_get("nonce")?;
    assert_ne!(encrypted_value, credential_secret.as_bytes());

    let vault_key = env::var("ASSET_GATEWAY_VAULT_KEY")
        .unwrap_or_else(|_| "asset-gw-dev-vault-key-32ch!".to_string());
    let vault = asset_gateway::core::vault::Vault::new(&vault_key);
    let decrypted_secret = vault.decrypt(&encrypted_value, &nonce)?;
    assert_eq!(decrypted_secret, credential_secret);

    // 3) 注册 Provider（验证 CRUD）
    let create_provider = client
        .post(format!("{}/api/providers", gateway_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({
            "id": &provider_id,
            "display_name": "Mock LLM Provider",
            "adapter": "llm_proxy",
            "asset_types": ["text"],
            "config": {
                "base_url": &mock_provider_url,
                "default_model": "gpt-5.4",
            },
            "priority": 100,
            "enabled": true,
        }))
        .send()
        .await?;
    assert_eq!(create_provider.status(), reqwest::StatusCode::OK);
    let create_provider_body: Value = create_provider.json().await?;
    assert_eq!(create_provider_body["ok"], json!(true));
    assert_eq!(create_provider_body["data"]["registered"], json!(true));

    let provider_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM providers WHERE id = $1")
        .bind(&provider_id)
        .fetch_one(&verify_pool)
        .await?;
    assert_eq!(provider_count, 1);

    let provider_health = client
        .get(format!(
            "{}/api/providers/{}/health",
            gateway_url, provider_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await?;
    assert_eq!(provider_health.status(), reqwest::StatusCode::OK);
    let provider_health_body: Value = provider_health.json().await?;
    assert_eq!(
        provider_health_body["data"]["health"]["healthy"],
        json!(true)
    );

    // 4) 发起 generate 请求
    let generate_response = client
        .post(format!("{}/api/generate", gateway_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({
            "asset_type": "text",
            "prompt": "hello from e2e",
            "model": "gpt-5.4",
            "provider": &provider_id,
            "params": {
                "stream": false,
            }
        }))
        .send()
        .await?;
    assert_eq!(generate_response.status(), reqwest::StatusCode::OK);
    let generate_body: Value = generate_response.json().await?;
    assert_eq!(generate_body["ok"], json!(true));
    assert_eq!(
        generate_body["data"]["provider_id"],
        json!(provider_id.as_str())
    );
    let output_data = generate_body["data"]["output_data"]
        .as_str()
        .context("generate output_data missing")?;
    assert!(output_data.contains("hello from e2e"));
    let job_id = generate_body["data"]["job_id"]
        .as_str()
        .context("job_id missing")?
        .to_string();

    // 5) 查询 job 状态
    let job_status = client
        .get(format!("{}/api/jobs/{}", gateway_url, job_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await?;
    assert_eq!(job_status.status(), reqwest::StatusCode::OK);
    let job_status_body: Value = job_status.json().await?;
    assert_eq!(job_status_body["ok"], json!(true));
    assert_eq!(job_status_body["data"]["status"], json!("completed"));
    assert_eq!(
        job_status_body["data"]["provider_id"],
        json!(provider_id.as_str())
    );

    // 6) 凭据脱敏验证
    let list_credentials = client
        .get(format!("{}/api/credentials", gateway_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await?;
    assert_eq!(list_credentials.status(), reqwest::StatusCode::OK);
    let list_credentials_body: Value = list_credentials.json().await?;
    let credentials = list_credentials_body["data"]["credentials"]
        .as_array()
        .context("credentials should be an array")?;
    let masked = credentials
        .iter()
        .find(|item| {
            item["key"].as_str() == Some("api_key")
                && item["provider_id"].as_str() == Some(provider_id.as_str())
        })
        .and_then(|item| item["value_masked"].as_str())
        .context("missing masked credential")?;
    assert_ne!(masked, credential_secret);
    assert!(!masked.contains(&credential_secret));
    assert!(masked.starts_with(&credential_secret[..4]));
    assert!(masked.ends_with(&credential_secret[credential_secret.len() - 4..]));

    // 7) 权限验证（非 admin 不能操作 credentials）
    let register_user = client
        .post(format!("{}/auth/register", gateway_url))
        .json(&json!({
            "username": &normal_username,
            "password": &normal_password,
        }))
        .send()
        .await?;
    assert_eq!(register_user.status(), reqwest::StatusCode::OK);
    let register_user_body: Value = register_user.json().await?;
    assert_eq!(register_user_body["data"]["role"], json!("user"));
    let normal_user_id = register_user_body["data"]["id"]
        .as_str()
        .context("missing normal user id")?
        .to_string();

    let approve_user = client
        .post(format!(
            "{}/api/users/{}/approve",
            gateway_url, normal_user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(approve_user.status(), reqwest::StatusCode::OK);

    let login_user = client
        .post(format!("{}/auth/login", gateway_url))
        .json(&json!({
            "username": &normal_username,
            "password": &normal_password,
        }))
        .send()
        .await?;
    assert_eq!(login_user.status(), reqwest::StatusCode::OK);
    let login_user_body: Value = login_user.json().await?;
    let user_token = login_user_body["data"]["token"]
        .as_str()
        .context("missing user token")?
        .to_string();

    let set_credential_by_user = client
        .put(format!("{}/api/credentials", gateway_url))
        .header("Authorization", format!("Bearer {}", user_token))
        .json(&json!({
            "key": "api_key",
            "value": "should-be-rejected",
            "provider_id": &provider_id,
        }))
        .send()
        .await?;
    assert_eq!(
        set_credential_by_user.status(),
        reqwest::StatusCode::FORBIDDEN
    );
    let forbidden_body: Value = set_credential_by_user.json().await?;
    assert_eq!(forbidden_body["ok"], json!(false));
    assert_eq!(forbidden_body["error"]["code"], json!("FORBIDDEN"));

    // Provider 删除（CRUD 的 D）
    let delete_provider = client
        .delete(format!("{}/api/providers/{}", gateway_url, provider_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await?;
    assert_eq!(delete_provider.status(), reqwest::StatusCode::OK);

    let provider_count_after_delete: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM providers WHERE id = $1")
            .bind(&provider_id)
            .fetch_one(&verify_pool)
            .await?;
    assert_eq!(provider_count_after_delete, 0);

    Ok(())
}
