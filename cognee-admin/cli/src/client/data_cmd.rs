use glob::Pattern;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

const UPLOAD_BATCH_SIZE: usize = 10;

pub async fn add(
    client: &CogneeClient,
    dataset_name: &str,
    content: &str,
) -> Result<Value, AppError> {
    client.add_data(dataset_name, content).await
}

pub async fn add_file(
    client: &CogneeClient,
    dataset_name: &str,
    file_path: &str,
) -> Result<Value, AppError> {
    let path = validate_file(file_path)?;
    client.upload_file(dataset_name, &path).await
}

pub async fn add_dir(
    client: &CogneeClient,
    dataset_name: &str,
    dir_path: &str,
    glob_pattern: &str,
) -> Result<Value, AppError> {
    let dir = validate_dir(dir_path)?;
    let pattern = Pattern::new(glob_pattern)
        .map_err(|err| AppError::Config(format!("Invalid glob pattern '{glob_pattern}': {err}")))?;
    let files = collect_matching_files(&dir, &pattern)?;

    if files.is_empty() {
        return Err(AppError::NotFound(format!(
            "No files matched '{}' in {}",
            glob_pattern,
            dir.display()
        )));
    }

    let progress = ProgressBar::new(files.len() as u64);
    if let Ok(style) = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} files",
    ) {
        progress.set_style(style.progress_chars("#>-"));
    }
    progress.set_message(dataset_name.to_owned());

    let mut uploaded = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    let total_batches = (files.len() + UPLOAD_BATCH_SIZE - 1) / UPLOAD_BATCH_SIZE;

    for (index, batch) in files.chunks(UPLOAD_BATCH_SIZE).enumerate() {
        match client.upload_files(dataset_name, batch).await {
            Ok(_) => {
                uploaded += batch.len();
                progress.inc(batch.len() as u64);
            }
            Err(batch_error) => {
                for file in batch {
                    match client.upload_file(dataset_name, file).await {
                        Ok(_) => uploaded += 1,
                        Err(file_error) => {
                            failed += 1;
                            errors.push(json!({
                                "path": file.display().to_string(),
                                "error": file_error.to_string(),
                            }));
                        }
                    }

                    progress.inc(1);
                }

                errors.push(json!({
                    "batch": batch
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>(),
                    "error": batch_error.to_string(),
                }));
            }
        }

        if index + 1 < total_batches {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    progress.finish_and_clear();

    Ok(json!({
        "dataset": dataset_name,
        "directory": dir.display().to_string(),
        "glob": glob_pattern,
        "batch_size": UPLOAD_BATCH_SIZE,
        "total": files.len(),
        "uploaded": uploaded,
        "failed": failed,
        "skipped": 0,
        "errors": errors,
    }))
}

pub async fn update(
    client: &CogneeClient,
    dataset_id: &str,
    data_id: &str,
    file_path: &str,
) -> Result<Value, AppError> {
    let path = validate_file(file_path)?;
    client.update_data(dataset_id, data_id, &path).await
}

pub async fn list(client: &CogneeClient, dataset_id: &str) -> Result<Value, AppError> {
    client.dataset_data(dataset_id).await
}

pub async fn delete(
    client: &CogneeClient,
    dataset_id: &str,
    data_id: &str,
) -> Result<Value, AppError> {
    client.delete_data(dataset_id, data_id).await
}

pub async fn raw(
    client: &CogneeClient,
    dataset_id: &str,
    data_id: &str,
) -> Result<Value, AppError> {
    client.raw_data(dataset_id, data_id).await
}

fn validate_file(file_path: &str) -> Result<PathBuf, AppError> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "File not found: {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(AppError::Config(format!(
            "Expected a file path: {}",
            path.display()
        )));
    }

    Ok(path)
}

fn validate_dir(dir_path: &str) -> Result<PathBuf, AppError> {
    let path = PathBuf::from(dir_path);
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Directory not found: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(AppError::Config(format!(
            "Expected a directory path: {}",
            path.display()
        )));
    }

    Ok(path)
}

fn collect_matching_files(dir: &Path, pattern: &Pattern) -> Result<Vec<PathBuf>, AppError> {
    let mut files = Vec::new();
    visit_dir(dir, dir, pattern, &mut files)?;
    files.sort();
    Ok(files)
}

fn visit_dir(
    root: &Path,
    current: &Path,
    pattern: &Pattern,
    files: &mut Vec<PathBuf>,
) -> Result<(), AppError> {
    let entries = std::fs::read_dir(current).map_err(|err| {
        AppError::Internal(anyhow::anyhow!(
            "Failed to read directory {}: {}",
            current.display(),
            err
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to read directory entry in {}: {}",
                current.display(),
                err
            ))
        })?;
        let path = entry.path();

        if path.is_dir() {
            visit_dir(root, &path, pattern, files)?;
            continue;
        }

        if path.is_file() && matches_pattern(root, &path, pattern) {
            files.push(path);
        }
    }

    Ok(())
}

fn matches_pattern(root: &Path, path: &Path, pattern: &Pattern) -> bool {
    path.strip_prefix(root)
        .ok()
        .map(|relative| pattern.matches_path(relative))
        .unwrap_or(false)
        || path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.to_str())
            .map(|relative| pattern.matches(relative))
            .unwrap_or(false)
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| pattern.matches(name))
            .unwrap_or(false)
}
