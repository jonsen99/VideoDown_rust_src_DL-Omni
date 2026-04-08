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
    pub playlist_items: Option<String>, // 新增：合集下载范围 (如 "1,3,5-7")
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed: f64,
    pub eta: u64,
    pub created_at: i64,
    pub error_msg: Option<String>,
}

impl Task {
    pub fn new(
        id: String,
        url: String,
        title: String,
        thumbnail: Option<String>,
        format_id: String,
        playlist_items: Option<String>, // 新增：支持创建任务时携带合集项
    ) -> Self {
        Self {
            id,
            url,
            title,
            thumbnail,
            status: TaskStatus::Pending,
            format_id,
            playlist_items,
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

// 新增：合集子项的简要信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistItem {
    pub playlist_index: Option<u32>,
    pub title: String,
    pub duration: Option<f64>,
    pub url: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaInfo {
    pub id: String,
    pub title: String,
    pub duration: f64,
    pub thumbnail: String,
    pub formats: Vec<MediaFormat>,
    pub playlist_entries: Option<Vec<PlaylistItem>>, // 新增：存储解析出的合集列表
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_download_path: String,
    pub max_concurrent_tasks: u8,
    pub max_threads_per_task: u8,
    pub proxy_url: Option<String>,
    pub theme: String,
    pub yt_dlp_version: Option<String>,
    pub split_audio_video: bool,
    pub video_quality: String,
    pub audio_quality: String,
    pub browser_cookie: Option<String>, // 新增：使用的浏览器 Cookie 名称 (如 "chrome")
    pub include_metadata: bool,         // 新增：是否生成独立文件夹并包含元数据
}