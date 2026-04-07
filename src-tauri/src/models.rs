use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Downloading,
    Paused,
    Merging,
    Error,
    Completed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub status: TaskStatus,
    pub format_id: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed: f64,
    pub eta: u64,
    pub created_at: i64,
    pub error_msg: Option<String>,
}

impl Task {
    pub fn new(id: String, url: String, format_id: String) -> Self {
        Self {
            id,
            url,
            title: String::from("解析中..."),
            thumbnail: None,
            status: TaskStatus::Pending,
            format_id,
            total_bytes: 0,
            downloaded_bytes: 0,
            speed: 0.0,
            eta: 0,
            created_at: chrono::Utc::now().timestamp_millis(),
            error_msg: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub filesize: Option<u64>,
    pub vcodec: String,
    pub acodec: String,
    pub format_note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaInfo {
    pub id: String,
    pub title: String,
    pub duration: f64,
    pub thumbnail: String,
    pub formats: Vec<MediaFormat>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_download_path: String,
    pub max_concurrent_tasks: u8,
    pub max_threads_per_task: u8,
    pub proxy_url: Option<String>,
    pub theme: String,
    pub yt_dlp_version: Option<String>,
    pub include_audio: bool, // 新增：是否同时下载音视频
}