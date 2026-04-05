use std::path::{Path, PathBuf};
use std::time::Instant;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

// -- Operations --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ProcessOp {
    SmartCrop {
        #[serde(default = "default_crop_mode")]
        mode: CropMode,
    },
    Resize {
        width: u32,
        height: u32,
    },
}

fn default_crop_mode() -> CropMode {
    CropMode::Tightest
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropMode {
    Tightest,
    PowerOf2,
}

// -- Request / Response --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRequest {
    pub input: String,
    pub operations: Vec<ProcessOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResponse {
    pub output_data: String,
    pub width: u32,
    pub height: u32,
    pub operations_applied: Vec<String>,
    pub elapsed_ms: u64,
}

// -- Pipeline --

pub struct Pipeline;

impl Pipeline {
    pub async fn run(req: &ProcessRequest) -> anyhow::Result<ProcessResponse> {
        let start = Instant::now();
        let tmp_dir = tempfile::tempdir()?;
        let mut current = Self::resolve_input(&req.input, tmp_dir.path()).await?;
        let mut applied = Vec::new();

        for op in &req.operations {
            let op_name = match op {
                ProcessOp::SmartCrop { mode } => format!("smart_crop:{:?}", mode),
                ProcessOp::Resize { width, height } => format!("resize:{}x{}", width, height),
            };

            current = match op {
                ProcessOp::SmartCrop { mode } => {
                    Self::smart_crop(&current, *mode, tmp_dir.path()).await?
                }
                ProcessOp::Resize { width, height } => {
                    Self::resize(&current, *width, *height, tmp_dir.path()).await?
                }
            };
            applied.push(op_name);
        }

        let bytes = tokio::fs::read(&current).await?;
        let output_data = STANDARD.encode(&bytes);
        let (width, height) = Self::get_dimensions(&current).await.unwrap_or((0, 0));

        Ok(ProcessResponse {
            output_data,
            width,
            height,
            operations_applied: applied,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn resolve_input(input: &str, tmp_dir: &Path) -> anyhow::Result<PathBuf> {
        if input.starts_with("http://") || input.starts_with("https://") {
            let resp = reqwest::get(input).await?;
            if !resp.status().is_success() {
                anyhow::bail!("failed to download input: HTTP {}", resp.status());
            }
            let bytes = resp.bytes().await?;
            let path = tmp_dir.join("input.png");
            tokio::fs::write(&path, &bytes).await?;
            Ok(path)
        } else if input.starts_with("data:") {
            let raw = input
                .find(";base64,")
                .map(|pos| &input[pos + 8..])
                .unwrap_or(input);
            let bytes = STANDARD.decode(raw)?;
            let path = tmp_dir.join("input.png");
            tokio::fs::write(&path, &bytes).await?;
            Ok(path)
        } else {
            let p = PathBuf::from(input);
            if !p.exists() {
                anyhow::bail!("input file not found: {}", input);
            }
            Ok(p)
        }
    }

    /// Detect ImageMagick command: `magick` (v7) or `convert` (v6).
    fn im_cmd() -> &'static str {
        // Prefer magick (v7), fallback to convert (v6)
        static CMD: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
        CMD.get_or_init(|| {
            if std::process::Command::new("magick")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
            {
                "magick"
            } else {
                "convert"
            }
        })
    }

    async fn smart_crop(input: &Path, mode: CropMode, tmp_dir: &Path) -> anyhow::Result<PathBuf> {
        let output = tmp_dir.join(format!("crop_{}.png", uuid::Uuid::new_v4()));
        let im = Self::im_cmd();

        match mode {
            CropMode::Tightest => {
                let result = Command::new(im)
                    .arg(input)
                    .args(["-trim", "+repage"])
                    .arg(&output)
                    .output()
                    .await?;
                if !result.status.success() {
                    anyhow::bail!(
                        "ImageMagick trim failed: {}",
                        String::from_utf8_lossy(&result.stderr)
                    );
                }
            }
            CropMode::PowerOf2 => {
                let trimmed = tmp_dir.join("trimmed_tmp.png");
                let result = Command::new(im)
                    .arg(input)
                    .args(["-trim", "+repage"])
                    .arg(&trimmed)
                    .output()
                    .await?;
                if !result.status.success() {
                    anyhow::bail!(
                        "ImageMagick trim failed: {}",
                        String::from_utf8_lossy(&result.stderr)
                    );
                }

                let (w, h) = Self::get_dimensions(&trimmed).await?;
                let new_w = w.next_power_of_two();
                let new_h = h.next_power_of_two();

                let result = Command::new(im)
                    .arg(&trimmed)
                    .args(["-gravity", "center", "-background", "none"])
                    .args(["-extent", &format!("{}x{}", new_w, new_h)])
                    .arg(&output)
                    .output()
                    .await?;
                if !result.status.success() {
                    anyhow::bail!(
                        "ImageMagick extent failed: {}",
                        String::from_utf8_lossy(&result.stderr)
                    );
                }
            }
        }
        Ok(output)
    }

    async fn resize(
        input: &Path,
        width: u32,
        height: u32,
        tmp_dir: &Path,
    ) -> anyhow::Result<PathBuf> {
        let output = tmp_dir.join(format!("resize_{}.png", uuid::Uuid::new_v4()));
        let result = Command::new(Self::im_cmd())
            .arg(input)
            .args(["-resize", &format!("{}x{}!", width, height)])
            .arg(&output)
            .output()
            .await?;
        if !result.status.success() {
            anyhow::bail!(
                "ImageMagick resize failed: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Ok(output)
    }

    async fn get_dimensions(path: &Path) -> anyhow::Result<(u32, u32)> {
        // `identify` works as standalone command in both v6 and v7
        let output = Command::new("identify")
            .args(["-format", "%wx%h"])
            .arg(path)
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!(
                "ImageMagick identify failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let text = String::from_utf8(output.stdout)?;
        let parts: Vec<&str> = text.trim().split('x').collect();
        if parts.len() != 2 {
            anyhow::bail!("unexpected identify output: {}", text);
        }
        Ok((parts[0].parse()?, parts[1].parse()?))
    }
}
