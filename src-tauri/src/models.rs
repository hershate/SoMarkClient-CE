use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

pub const DEFAULT_BASE_URL: &str = "https://somark.ai/api/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskKind {
    Parse,
    Office,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskMode {
    Auto,
    Sync,
    Async,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Queued,
    Uploading,
    SubmissionUnknown,
    Waiting,
    Stopped,
    Paused,
    Downloading,
    Success,
    PartialSuccess,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Uploading => "uploading",
            Self::SubmissionUnknown => "submissionUnknown",
            Self::Waiting => "waiting",
            Self::Stopped => "stopped",
            Self::Paused => "paused",
            Self::Downloading => "downloading",
            Self::Success => "success",
            Self::PartialSuccess => "partialSuccess",
            Self::Failed => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::PartialSuccess | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TaskSource {
    Local { path: String },
    Url { url: String },
    Clipboard { path: String },
}

impl TaskSource {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Local { path } | Self::Clipboard { path } => Some(Path::new(path)),
            Self::Url { .. } => None,
        }
    }

    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Url { url } => Some(url),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementFormats {
    pub image: String,
    pub formula: String,
    pub table: String,
    pub cs: String,
}

impl Default for ElementFormats {
    fn default() -> Self {
        Self {
            image: "file".into(),
            formula: "latex".into(),
            table: "markdown".into(),
            cs: "image".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureConfig {
    pub enable_text_cross_page: bool,
    pub enable_table_cross_page: bool,
    pub enable_title_level_recognition: bool,
    pub enable_inline_image: bool,
    pub enable_table_image: bool,
    pub enable_image_understanding: bool,
    pub enable_image_ocr: bool,
    pub keep_header_footer: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            enable_text_cross_page: true,
            enable_table_cross_page: true,
            enable_title_level_recognition: true,
            enable_inline_image: true,
            enable_table_image: true,
            enable_image_understanding: true,
            enable_image_ocr: false,
            keep_header_footer: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParseOptions {
    pub element_formats: ElementFormats,
    pub feature_config: FeatureConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeOptions {
    pub target_type: String,
    pub layout_mode: String,
}

impl Default for OfficeOptions {
    fn default() -> Self {
        Self {
            target_type: "word".into(),
            layout_mode: "flow".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputFile {
    pub format: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: String,
    pub message: String,
    pub code: Option<i32>,
    pub retryable: bool,
}

impl AppError {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            code: None,
            retryable: false,
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self {
            kind: "network".into(),
            message: message.into(),
            code: None,
            retryable: true,
        }
    }

    pub fn api(code: i32, message: impl Into<String>) -> Self {
        let retryable = matches!(code, 1101 | 1124);
        let kind = match code {
            1107 => "authentication",
            1108 | 1123 => "quota",
            1101 | 1124 => "rateLimit",
            1104 | 1106 | 1111 | 1118 | 1132 => "file",
            1102 => "timeout",
            1120 | 1137 | 1138 | 1144 => "task",
            _ => "api",
        };
        Self {
            kind: kind.into(),
            message: message.into(),
            code: Some(code),
            retryable,
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: "timeout".into(),
            message: message.into(),
            code: None,
            retryable: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub batch_id: String,
    pub name: String,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub mode: TaskMode,
    pub source: TaskSource,
    pub formats: Vec<String>,
    pub parse_options: ParseOptions,
    pub office_options: OfficeOptions,
    pub service_id: String,
    pub remote_id: Option<String>,
    pub remote_status: Option<String>,
    pub progress: f64,
    pub file_size: u64,
    pub page_count: Option<u32>,
    pub output_root: String,
    pub outputs: Vec<OutputFile>,
    pub result_path: Option<String>,
    pub error: Option<AppError>,
    pub retry_count: u32,
    pub priority: i64,
    pub created_at: String,
    pub updated_at: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub name: String,
    pub kind: TaskKind,
    pub mode: TaskMode,
    pub source: TaskSource,
    pub formats: Vec<String>,
    #[serde(default)]
    pub parse_options: ParseOptions,
    #[serde(default)]
    pub office_options: OfficeOptions,
    pub service_id: String,
    pub output_root: String,
    #[serde(default)]
    pub file_size: u64,
    pub page_count: Option<u32>,
}

impl NewTask {
    pub fn into_task(self, batch_id: String, priority: i64) -> Task {
        let now = Utc::now().to_rfc3339();
        Task {
            id: Uuid::new_v4().to_string(),
            batch_id,
            name: self.name,
            kind: self.kind,
            status: TaskStatus::Queued,
            mode: self.mode,
            source: self.source,
            formats: self.formats,
            parse_options: self.parse_options,
            office_options: self.office_options,
            service_id: self.service_id,
            remote_id: None,
            remote_status: None,
            progress: 0.0,
            file_size: self.file_size,
            page_count: self.page_count,
            output_root: self.output_root,
            outputs: Vec::new(),
            result_path: None,
            error: None,
            retry_count: 0,
            priority,
            created_at: now.clone(),
            updated_at: now,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxyMode {
    System,
    Direct,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub proxy_mode: ProxyMode,
    pub proxy_url: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub max_file_size_mb: u64,
    pub max_pages: u32,
    pub qps: u32,
    pub max_concurrency: usize,
    pub higher_concurrency_approved: bool,
}

impl ServiceProfile {
    pub fn official() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "SoMark".into(),
            base_url: DEFAULT_BASE_URL.into(),
            proxy_mode: ProxyMode::System,
            proxy_url: None,
            timeout_secs: 120,
            max_retries: 3,
            max_file_size_mb: 200,
            max_pages: 300,
            qps: 4,
            max_concurrency: 1,
            higher_concurrency_approved: false,
        }
    }

    pub fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(alias = "remaining_paid_pages")]
    pub remaining_paid_pages: i64,
    #[serde(alias = "remaining_free_pages_today")]
    pub remaining_free_pages_today: i64,
    #[serde(alias = "remaining_free_pages_this_month")]
    pub remaining_free_pages_this_month: i64,
    #[serde(alias = "dashboard_url")]
    pub dashboard_url: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub theme: String,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub sound: bool,
    pub history_days: u32,
    pub log_days: u32,
    pub default_output_root: String,
    pub concurrency: usize,
    pub retry_limit: u32,
    pub quota_warning_pages: i64,
    pub block_on_low_quota: bool,
    pub active_service_id: Option<String>,
}

impl AppSettings {
    pub fn new(output_root: String) -> Self {
        Self {
            language: "zh-CN".into(),
            theme: "system".into(),
            close_to_tray: true,
            notifications: true,
            sound: true,
            history_days: 90,
            log_days: 14,
            default_output_root: output_root,
            concurrency: 1,
            retry_limit: 3,
            quota_warning_pages: 20,
            block_on_low_quota: false,
            active_service_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiMetadata {
    #[serde(alias = "page_num")]
    pub page_num: Option<u32>,
    #[serde(alias = "file_type")]
    pub file_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParseOutputs {
    pub markdown: Option<String>,
    #[serde(rename = "json")]
    pub json_output: Option<Value>,
    pub zip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    #[serde(alias = "file_name")]
    pub file_name: Option<String>,
    pub catalog: Option<Value>,
    pub outputs: ParseOutputs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParseData {
    #[serde(alias = "task_id")]
    pub task_id: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<ApiMetadata>,
    pub result: Option<ParseResult>,
    #[serde(alias = "file_name")]
    pub file_name: Option<String>,
    #[serde(alias = "error_code")]
    pub error_code: Option<i32>,
    #[serde(alias = "error_message")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OfficeData {
    #[serde(alias = "task_id")]
    pub task_id: Option<String>,
    pub status: Option<String>,
    #[serde(alias = "target_type")]
    pub target_type: Option<String>,
    #[serde(alias = "layout_mode")]
    pub layout_mode: Option<String>,
    #[serde(alias = "file_name")]
    pub file_name: Option<String>,
    #[serde(alias = "page_num")]
    pub page_num: Option<u32>,
    #[serde(alias = "result_file_name")]
    pub result_file_name: Option<String>,
    #[serde(alias = "error_code")]
    pub error_code: Option<i32>,
    #[serde(alias = "error_message")]
    pub error_message: Option<String>,
    #[serde(alias = "download_url")]
    pub download_url: Option<String>,
    #[serde(alias = "expires_in")]
    pub expires_in: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredResult {
    pub parse: Option<ParseResult>,
    pub office: Option<OfficeData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResult {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub page_count: Option<u32>,
    pub accepted: bool,
    pub issues: Vec<String>,
}
