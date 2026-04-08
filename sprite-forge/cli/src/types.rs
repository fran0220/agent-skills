use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationType {
    Walk,
    Run,
    Attack,
    Idle,
    Death,
    #[serde(untagged)]
    Custom(String),
}

impl AnimationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Walk => "walk",
            Self::Run => "run",
            Self::Attack => "attack",
            Self::Idle => "idle",
            Self::Death => "death",
            Self::Custom(value) => value.as_str(),
        }
    }
}

impl From<String> for AnimationType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "walk" => Self::Walk,
            "run" => Self::Run,
            "attack" => Self::Attack,
            "idle" => Self::Idle,
            "death" => Self::Death,
            _ => Self::Custom(value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Right,
    Left,
    Front,
    Back,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Left => "left",
            Self::Front => "front",
            Self::Back => "back",
        }
    }
}

impl From<String> for Direction {
    fn from(value: String) -> Self {
        match value.as_str() {
            "left" => Self::Left,
            "front" => Self::Front,
            "back" => Self::Back,
            _ => Self::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum GridSize {
    #[serde(rename = "2x2")]
    Grid2x2,
    #[default]
    #[serde(rename = "3x3")]
    Grid3x3,
    #[serde(rename = "4x4")]
    Grid4x4,
}

impl GridSize {
    pub fn cols(&self) -> u32 {
        match self {
            Self::Grid2x2 => 2,
            Self::Grid3x3 => 3,
            Self::Grid4x4 => 4,
        }
    }

    pub fn rows(&self) -> u32 {
        self.cols()
    }

    #[allow(dead_code)]
    pub fn total_frames(&self) -> u32 {
        self.cols() * self.rows()
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Grid2x2 => "2x2",
            Self::Grid3x3 => "3x3",
            Self::Grid4x4 => "4x4",
        }
    }
}

impl From<String> for GridSize {
    fn from(value: String) -> Self {
        match value.as_str() {
            "2x2" => Self::Grid2x2,
            "4x4" => Self::Grid4x4,
            _ => Self::Grid3x3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteRequest {
    pub character_description: Option<String>,
    pub character_image: Option<String>,
    pub animation_type: AnimationType,
    #[serde(default)]
    pub direction: Direction,
    #[serde(default)]
    pub grid_size: GridSize,
    pub style: Option<String>,
    #[serde(default = "default_true")]
    pub remove_background: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteResponse {
    pub job_id: String,
    pub status: JobStatus,
    pub sprite_sheet_path: Option<String>,
    pub individual_frames: Vec<String>,
    pub grid_image_path: Option<String>,
    pub enhanced_prompt: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    PromptEnhancing,
    Generating,
    Processing,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::PromptEnhancing => "prompt_enhancing",
            Self::Generating => "generating",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl From<String> for JobStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "prompt_enhancing" => Self::PromptEnhancing,
            "generating" => Self::Generating,
            "processing" => Self::Processing,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    User,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
        }
    }
}

impl From<String> for UserRole {
    fn from(value: String) -> Self {
        if value == "admin" {
            Self::Admin
        } else {
            Self::User
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub daily_quota: i64,
    pub is_banned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub user: User,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

impl AuthClaims {
    pub fn new(user: &User) -> Self {
        let now = Utc::now();
        let exp = now + Duration::days(7);
        Self {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role.as_str().to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeResponse {
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub user_id: String,
    pub status: JobStatus,
    pub character_description: Option<String>,
    pub character_image_url: Option<String>,
    pub animation_type: AnimationType,
    pub direction: Direction,
    pub grid_size: GridSize,
    pub style: Option<String>,
    pub remove_background: bool,
    pub enhanced_prompt: Option<String>,
    pub error_message: Option<String>,
    pub output_dir: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl Job {
    pub fn to_sprite_request(&self) -> SpriteRequest {
        SpriteRequest {
            character_description: self.character_description.clone(),
            character_image: self.character_image_url.clone(),
            animation_type: self.animation_type.clone(),
            direction: self.direction.clone(),
            grid_size: self.grid_size,
            style: self.style.clone(),
            remove_background: self.remove_background,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateJobRequest {
    pub character_description: Option<String>,
    pub character_image_url: Option<String>,
    pub animation_type: AnimationType,
    #[serde(default)]
    pub direction: Direction,
    #[serde(default)]
    pub grid_size: GridSize,
    pub style: Option<String>,
    #[serde(default = "default_true")]
    pub remove_background: bool,
}

impl CreateJobRequest {
    pub fn into_job_insert(self, user_id: String, job_id: String) -> NewJob {
        NewJob {
            id: job_id,
            user_id,
            character_description: self.character_description,
            character_image_url: self.character_image_url,
            animation_type: self.animation_type,
            direction: self.direction,
            grid_size: self.grid_size,
            style: self.style,
            remove_background: self.remove_background,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewJob {
    pub id: String,
    pub user_id: String,
    pub character_description: Option<String>,
    pub character_image_url: Option<String>,
    pub animation_type: AnimationType,
    pub direction: Direction,
    pub grid_size: GridSize,
    pub style: Option<String>,
    pub remove_background: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateJobResponse {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobListResponse {
    pub jobs: Vec<Job>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobProgress {
    pub job_id: String,
    pub status: String,
    pub progress_pct: u8,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminUserListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminJobsQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub status: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUsersResponse {
    pub users: Vec<User>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminUpdateUserRequest {
    pub name: Option<String>,
    pub role: Option<UserRole>,
    pub daily_quota: Option<i64>,
    pub is_banned: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminStatsResponse {
    pub total_users: u64,
    pub active_users_today: u64,
    pub total_jobs: u64,
    pub pending_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigEntry {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSystemConfigRequest {
    pub items: Vec<SystemConfigValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemConfigResponse {
    pub items: Vec<SystemConfigEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetryJobResponse {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: u64,
    pub limit: u64,
}

impl Pagination {
    pub fn new(page: u64, limit: u64) -> Self {
        Self {
            page: page.max(1),
            limit: limit.clamp(1, 100),
        }
    }

    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.limit
    }
}

fn default_page() -> u64 {
    1
}

fn default_limit() -> u64 {
    20
}


