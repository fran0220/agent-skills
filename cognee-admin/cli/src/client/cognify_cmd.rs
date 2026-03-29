use serde_json::{Map, Value};
use std::path::Path;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn run(
    client: &CogneeClient,
    dataset_id: Option<&str>,
    dataset_name: Option<&str>,
    custom_prompt: Option<&str>,
    custom_prompt_file: Option<&Path>,
    background: bool,
    chunks_per_batch: Option<u32>,
) -> Result<Value, AppError> {
    if dataset_id.is_some() && dataset_name.is_some() {
        return Err(AppError::Config(
            "use either --dataset-id or --dataset-name, not both".into(),
        ));
    }

    if custom_prompt.is_some() && custom_prompt_file.is_some() {
        return Err(AppError::Config(
            "use either --custom-prompt or --custom-prompt-file, not both".into(),
        ));
    }

    let mut payload = Map::new();

    if let Some(dataset) = dataset_id.or(dataset_name) {
        payload.insert(
            "datasets".into(),
            Value::Array(vec![Value::String(dataset.to_string())]),
        );
    }

    if let Some(prompt) = resolve_custom_prompt(custom_prompt, custom_prompt_file)? {
        payload.insert("custom_prompt".into(), Value::String(prompt));
    }

    payload.insert("run_in_background".into(), Value::Bool(background));

    if let Some(chunks_per_batch) = chunks_per_batch {
        payload.insert(
            "chunks_per_batch".into(),
            Value::Number(chunks_per_batch.into()),
        );
    }

    client.cognify(Value::Object(payload)).await
}

fn resolve_custom_prompt(
    custom_prompt: Option<&str>,
    custom_prompt_file: Option<&Path>,
) -> Result<Option<String>, AppError> {
    if let Some(prompt) = custom_prompt {
        return Ok(Some(prompt.to_string()));
    }

    let Some(path) = custom_prompt_file else {
        return Ok(None);
    };

    std::fs::read_to_string(path).map(Some).map_err(|err| {
        AppError::Config(format!(
            "failed to read custom prompt file {}: {err}",
            path.display()
        ))
    })
}
