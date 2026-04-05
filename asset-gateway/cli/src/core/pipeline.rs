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
    Compose {
        direction: ComposeDirection,
        #[serde(default)]
        columns: Option<u32>,
        #[serde(default)]
        padding: Option<u32>,
        #[serde(default)]
        frame_width: Option<u32>,
        #[serde(default)]
        frame_height: Option<u32>,
    },
    ExtractFrames {
        #[serde(default = "default_frame_count")]
        count: u32,
    },
    RemoveBg {
        #[serde(default = "default_bg_color")]
        bg_color: Option<String>,
    },
}

fn default_crop_mode() -> CropMode {
    CropMode::Tightest
}

fn default_frame_count() -> u32 {
    8
}

fn default_bg_color() -> Option<String> {
    None
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropMode {
    Tightest,
    PowerOf2,
}

impl CropMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tightest => "tightest",
            Self::PowerOf2 => "power_of2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposeDirection {
    Horizontal,
    Vertical,
    Grid,
}

impl ComposeDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Grid => "grid",
        }
    }
}

// -- Request / Response --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRequest {
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
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

enum PipelineState {
    Single(PathBuf),
    Multi(Vec<PathBuf>),
}

impl PipelineState {
    fn into_single(self, op_name: &str) -> anyhow::Result<PathBuf> {
        match self {
            Self::Single(path) => Ok(path),
            Self::Multi(paths) => anyhow::bail!(
                "operation {} requires a single input, got {} inputs",
                op_name,
                paths.len()
            ),
        }
    }

    fn into_multi(self) -> Vec<PathBuf> {
        match self {
            Self::Single(path) => vec![path],
            Self::Multi(paths) => paths,
        }
    }
}

impl Pipeline {
    pub async fn run(req: &ProcessRequest) -> anyhow::Result<ProcessResponse> {
        let start = Instant::now();
        let tmp_dir = tempfile::tempdir()?;
        let mut current = Self::resolve_inputs(req, tmp_dir.path()).await?;
        let mut applied = Vec::new();

        for op in &req.operations {
            let op_name = match op {
                ProcessOp::SmartCrop { mode } => format!("smart_crop:{}", mode.as_str()),
                ProcessOp::Resize { width, height } => format!("resize:{}x{}", width, height),
                ProcessOp::Compose {
                    direction,
                    columns,
                    padding,
                    frame_width,
                    frame_height,
                } => format!(
                    "compose:{}:{}:{}:{}x{}",
                    direction.as_str(),
                    columns
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "auto".to_string()),
                    padding.unwrap_or(0),
                    frame_width
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "auto".to_string()),
                    frame_height
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "auto".to_string()),
                ),
                ProcessOp::ExtractFrames { count } => format!("extract_frames:{}", count),
                ProcessOp::RemoveBg { bg_color } => format!(
                    "remove_bg:{}",
                    bg_color.as_deref().unwrap_or("auto")
                ),
            };

            current = match op {
                ProcessOp::SmartCrop { mode } => {
                    let input = current.into_single("smart_crop")?;
                    PipelineState::Single(Self::smart_crop(&input, *mode, tmp_dir.path()).await?)
                }
                ProcessOp::Resize { width, height } => {
                    let input = current.into_single("resize")?;
                    PipelineState::Single(
                        Self::resize(&input, *width, *height, tmp_dir.path()).await?,
                    )
                }
                ProcessOp::Compose {
                    direction,
                    columns,
                    padding,
                    frame_width,
                    frame_height,
                } => PipelineState::Single(
                    Self::compose(
                        current.into_multi(),
                        *direction,
                        *columns,
                        *padding,
                        *frame_width,
                        *frame_height,
                        tmp_dir.path(),
                    )
                    .await?,
                ),
                ProcessOp::ExtractFrames { count } => {
                    let input = current.into_single("extract_frames")?;
                    PipelineState::Multi(
                        Self::extract_frames(&input, *count, tmp_dir.path()).await?,
                    )
                }
                ProcessOp::RemoveBg { bg_color } => {
                    let paths = current.into_multi();
                    let mut results = Vec::with_capacity(paths.len());
                    for path in &paths {
                        results.push(
                            Self::remove_bg(path, bg_color.as_deref(), tmp_dir.path()).await?,
                        );
                    }
                    if results.len() == 1 {
                        PipelineState::Single(results.remove(0))
                    } else {
                        PipelineState::Multi(results)
                    }
                }
            };
            applied.push(op_name);
        }

        let current = current.into_single("finalize")?;
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

    async fn resolve_inputs(req: &ProcessRequest, tmp_dir: &Path) -> anyhow::Result<PipelineState> {
        if !req.inputs.is_empty() {
            let mut inputs = Vec::with_capacity(req.inputs.len());
            for input in &req.inputs {
                inputs.push(Self::resolve_input(input, tmp_dir).await?);
            }
            return Ok(PipelineState::Multi(inputs));
        }

        if let Some(input) = &req.input {
            return Ok(PipelineState::Single(
                Self::resolve_input(input, tmp_dir).await?,
            ));
        }

        anyhow::bail!("either input or inputs must be provided")
    }

    async fn resolve_input(input: &str, tmp_dir: &Path) -> anyhow::Result<PathBuf> {
        if input.starts_with("http://") || input.starts_with("https://") {
            let resp = reqwest::get(input).await?;
            if !resp.status().is_success() {
                anyhow::bail!("failed to download input: HTTP {}", resp.status());
            }
            let bytes = resp.bytes().await?;
            let path = tmp_dir.join(format!("input_{}.png", uuid::Uuid::new_v4()));
            tokio::fs::write(&path, &bytes).await?;
            Ok(path)
        } else if input.starts_with("data:") {
            let raw = input
                .find(";base64,")
                .map(|pos| &input[pos + 8..])
                .unwrap_or(input);
            let bytes = STANDARD.decode(raw)?;
            let path = tmp_dir.join(format!("input_{}.png", uuid::Uuid::new_v4()));
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

    async fn run_command(command: &mut Command, error_prefix: &str) -> anyhow::Result<()> {
        let result = command.output().await?;
        if !result.status.success() {
            anyhow::bail!(
                "{}: {}",
                error_prefix,
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Ok(())
    }

    async fn smart_crop(input: &Path, mode: CropMode, tmp_dir: &Path) -> anyhow::Result<PathBuf> {
        let output = tmp_dir.join(format!("crop_{}.png", uuid::Uuid::new_v4()));
        let im = Self::im_cmd();

        match mode {
            CropMode::Tightest => {
                let mut command = Command::new(im);
                command.arg(input).args(["-trim", "+repage"]).arg(&output);
                Self::run_command(&mut command, "ImageMagick trim failed").await?;
            }
            CropMode::PowerOf2 => {
                let trimmed = tmp_dir.join(format!("trimmed_{}.png", uuid::Uuid::new_v4()));
                let mut trim_command = Command::new(im);
                trim_command
                    .arg(input)
                    .args(["-trim", "+repage"])
                    .arg(&trimmed);
                Self::run_command(&mut trim_command, "ImageMagick trim failed").await?;

                let (w, h) = Self::get_dimensions(&trimmed).await?;
                let new_w = w.next_power_of_two();
                let new_h = h.next_power_of_two();

                let mut extent_command = Command::new(im);
                extent_command
                    .arg(&trimmed)
                    .args(["-gravity", "center", "-background", "none"])
                    .args(["-extent", &format!("{}x{}", new_w, new_h)])
                    .arg(&output);
                Self::run_command(&mut extent_command, "ImageMagick extent failed").await?;
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
        let mut command = Command::new(Self::im_cmd());
        command
            .arg(input)
            .args(["-resize", &format!("{}x{}!", width, height)])
            .arg(&output);
        Self::run_command(&mut command, "ImageMagick resize failed").await?;
        Ok(output)
    }

    async fn compose(
        inputs: Vec<PathBuf>,
        direction: ComposeDirection,
        columns: Option<u32>,
        padding: Option<u32>,
        frame_width: Option<u32>,
        frame_height: Option<u32>,
        tmp_dir: &Path,
    ) -> anyhow::Result<PathBuf> {
        if inputs.is_empty() {
            anyhow::bail!("compose requires at least one input")
        }

        let normalized = match (frame_width, frame_height) {
            (Some(width), Some(height)) => {
                let mut normalized = Vec::with_capacity(inputs.len());
                for input in inputs {
                    let cropped = Self::smart_crop(&input, CropMode::Tightest, tmp_dir).await?;
                    normalized.push(Self::resize(&cropped, width, height, tmp_dir).await?);
                }
                normalized
            }
            (None, None) => inputs,
            _ => anyhow::bail!("frame_width and frame_height must be provided together"),
        };

        let output = tmp_dir.join(format!("compose_{}.png", uuid::Uuid::new_v4()));
        let padding = padding.unwrap_or(0);

        match direction {
            ComposeDirection::Horizontal => {
                let mut command = Command::new(Self::im_cmd());
                for input in &normalized {
                    command.arg(input);
                }
                command.arg("+append").arg(&output);
                Self::run_command(&mut command, "ImageMagick horizontal compose failed").await?;
            }
            ComposeDirection::Vertical => {
                let mut command = Command::new(Self::im_cmd());
                for input in &normalized {
                    command.arg(input);
                }
                command.arg("-append").arg(&output);
                Self::run_command(&mut command, "ImageMagick vertical compose failed").await?;
            }
            ComposeDirection::Grid => {
                let columns =
                    columns.unwrap_or_else(|| Self::default_grid_columns(normalized.len()));
                if columns == 0 {
                    anyhow::bail!("columns must be greater than 0")
                }

                let geometry = match (frame_width, frame_height) {
                    (Some(width), Some(height)) => {
                        format!("{}x{}+{}+{}", width, height, padding, padding)
                    }
                    _ => format!("+{}+{}", padding, padding),
                };

                let mut command = Command::new("montage");
                for input in &normalized {
                    command.arg(input);
                }
                command
                    .args(["-tile", &format!("{}x", columns)])
                    .args(["-geometry", &geometry])
                    .args(["-background", "none"])
                    .arg(&output);
                Self::run_command(&mut command, "ImageMagick grid compose failed").await?;
            }
        }

        Ok(output)
    }

    async fn extract_frames(
        input: &Path,
        count: u32,
        tmp_dir: &Path,
    ) -> anyhow::Result<Vec<PathBuf>> {
        // Get total frame count using ffprobe
        let probe = Command::new("ffprobe")
            .args([
                "-v", "error",
                "-select_streams", "v:0",
                "-count_packets",
                "-show_entries", "stream=nb_read_packets",
                "-of", "csv=p=0",
            ])
            .arg(input)
            .output()
            .await?;
        if !probe.status.success() {
            anyhow::bail!(
                "ffprobe failed: {}",
                String::from_utf8_lossy(&probe.stderr)
            );
        }
        let total_frames: u32 = String::from_utf8(probe.stdout)?
            .trim()
            .parse()
            .unwrap_or(0);
        if total_frames == 0 {
            anyhow::bail!("ffprobe returned 0 frames for input video");
        }

        // Calculate select interval: pick `count` evenly spaced frames
        let interval = if total_frames <= count {
            1
        } else {
            total_frames / count
        };

        let pattern = tmp_dir.join("frame_%04d.png");
        let mut command = Command::new("ffmpeg");
        command
            .args(["-i"])
            .arg(input)
            .args([
                "-vf",
                &format!("select='not(mod(n\\,{}))'", interval),
                "-vsync", "0",
                "-frames:v", &count.to_string(),
            ])
            .arg(&pattern);
        Self::run_command(&mut command, "ffmpeg extract_frames failed").await?;

        // Collect output frames in order
        let mut frames = Vec::new();
        for i in 1..=count {
            let path = tmp_dir.join(format!("frame_{:04}.png", i));
            if path.exists() {
                frames.push(path);
            }
        }
        if frames.is_empty() {
            anyhow::bail!("ffmpeg produced no output frames");
        }
        Ok(frames)
    }

    async fn remove_bg(
        input: &Path,
        _bg_color: Option<&str>,
        tmp_dir: &Path,
    ) -> anyhow::Result<PathBuf> {
        let output = tmp_dir.join(format!("nobg_{}.png", uuid::Uuid::new_v4()));

        let file_bytes = tokio::fs::read(input).await?;
        let ext = input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let b64 = STANDARD.encode(&file_bytes);
        let data_uri = format!("data:{};base64,{}", mime, b64);
        drop(file_bytes);

        let bgsweep_url = std::env::var("BGSWEEP_URL")
            .unwrap_or_else(|_| "https://bgsweep.com/api/remove-background".to_string());

        let client = reqwest::Client::new();
        let resp = client
            .post(&bgsweep_url)
            .header("Content-Type", "application/json")
            .body(format!(r#"{{"imageData":"{}"}}"#, data_uri))
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("bgsweep error: HTTP {} - {}", status, body);
        }

        let result: serde_json::Value = resp.json().await?;
        if !result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let err = result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!("bgsweep failed: {}", err);
        }

        let output_url = result
            .get("outputUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("bgsweep: missing outputUrl"))?;

        let img_resp = client
            .get(output_url)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;
        let png_bytes = img_resp.bytes().await?;
        tokio::fs::write(&output, &png_bytes).await?;

        Ok(output)
    }

    fn default_grid_columns(input_count: usize) -> u32 {
        (input_count as f64).sqrt().ceil() as u32
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

#[cfg(test)]
mod tests {
    use super::{ComposeDirection, ProcessRequest};
    use serde_json::json;

    #[test]
    fn compose_direction_deserializes_from_snake_case() {
        let direction: ComposeDirection = serde_json::from_str("\"horizontal\"").unwrap();
        assert_eq!(direction, ComposeDirection::Horizontal);

        let direction: ComposeDirection = serde_json::from_str("\"vertical\"").unwrap();
        assert_eq!(direction, ComposeDirection::Vertical);

        let direction: ComposeDirection = serde_json::from_str("\"grid\"").unwrap();
        assert_eq!(direction, ComposeDirection::Grid);
    }

    #[test]
    fn process_request_accepts_legacy_single_input_shape() {
        let request: ProcessRequest = serde_json::from_value(json!({
            "input": "input.png",
            "operations": [{ "op": "resize", "width": 32, "height": 32 }],
        }))
        .unwrap();

        assert_eq!(request.input.as_deref(), Some("input.png"));
        assert!(request.inputs.is_empty());
    }

    #[test]
    fn process_request_accepts_multi_input_shape() {
        let request: ProcessRequest = serde_json::from_value(json!({
            "inputs": ["a.png", "b.png"],
            "operations": [{ "op": "compose", "direction": "horizontal" }],
        }))
        .unwrap();

        assert!(request.input.is_none());
        assert_eq!(request.inputs, vec!["a.png", "b.png"]);
    }

    #[tokio::test]
    async fn compose_horizontal_produces_wider_output() {
        use super::Pipeline;
        use std::process::Command as StdCommand;

        // Skip if ImageMagick not available
        let magick_ok = StdCommand::new("magick")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
            || StdCommand::new("convert")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success());
        if !magick_ok {
            eprintln!("skipping compose test: ImageMagick not available");
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let im = Pipeline::im_cmd();
        // Create 3 solid-color 32x32 PNGs
        for (i, color) in ["red", "green", "blue"].iter().enumerate() {
            let path = tmp.path().join(format!("img{}.png", i));
            StdCommand::new(im)
                .args(["-size", "32x32", &format!("xc:{}", color)])
                .arg(&path)
                .output()
                .expect("create test image");
        }

        let req = ProcessRequest {
            input: None,
            inputs: (0..3)
                .map(|i| tmp.path().join(format!("img{}.png", i)).to_string_lossy().to_string())
                .collect(),
            operations: vec![super::ProcessOp::Compose {
                direction: ComposeDirection::Horizontal,
                columns: None,
                padding: None,
                frame_width: None,
                frame_height: None,
            }],
        };

        let result = Pipeline::run(&req).await.unwrap();
        assert_eq!(result.width, 96); // 3 * 32
        assert_eq!(result.height, 32);
        assert!(!result.output_data.is_empty());
        assert_eq!(result.operations_applied, vec!["compose:horizontal:auto:0:autoxauto"]);
    }

    #[tokio::test]
    async fn compose_grid_with_normalization() {
        use super::Pipeline;
        use std::process::Command as StdCommand;

        let magick_ok = StdCommand::new("magick")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
            || StdCommand::new("convert")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success());
        if !magick_ok {
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let im = Pipeline::im_cmd();
        // Create 4 images of different sizes
        for (i, (w, h)) in [(50, 80), (30, 40), (60, 60), (45, 70)].iter().enumerate() {
            let path = tmp.path().join(format!("img{}.png", i));
            StdCommand::new(im)
                .args(["-size", &format!("{}x{}", w, h), "xc:orange", "-alpha", "set"])
                .arg(&path)
                .output()
                .expect("create test image");
        }

        let req = ProcessRequest {
            input: None,
            inputs: (0..4)
                .map(|i| tmp.path().join(format!("img{}.png", i)).to_string_lossy().to_string())
                .collect(),
            operations: vec![super::ProcessOp::Compose {
                direction: ComposeDirection::Grid,
                columns: Some(2),
                padding: Some(0),
                frame_width: Some(32),
                frame_height: Some(32),
            }],
        };

        let result = Pipeline::run(&req).await.unwrap();
        // 2x2 grid of 32x32 = 64x64
        assert_eq!(result.width, 64);
        assert_eq!(result.height, 64);
        assert!(!result.output_data.is_empty());
    }
}
