use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use crate::models::Config;

pub struct ConfigManager {
    config_path: PathBuf,
    pub settings: Config,
}

impl ConfigManager {
    /// 初始化配置管理器
    pub fn init(app: &AppHandle) -> Result<Self, String> {
        let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;

        let config_path = app_dir.join("config.json");

        // 如果配置文件存在，则读取并反序列化；否则生成默认配置
        // 若存在旧配置文件缺失新增的字段，反序列化会自动 fallback 到 default_config 兜底填充
        let settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default_config(app))
        } else {
            Self::default_config(app)
        };

        // 重新写回配置，确保向旧用户补充缺少的字段 (如 browser_cookie 等)
        let content = serde_json::to_string_pretty(&settings).unwrap();
        let _ = fs::write(&config_path, content);

        Ok(Self {
            config_path,
            settings,
        })
    }

    /// 获取默认配置
    fn default_config(app: &AppHandle) -> Config {
        let download_path = app.path().download_dir()
            .or_else(|_| app.path().document_dir())
            .unwrap_or_else(|_| PathBuf::from("./"))
            .to_string_lossy()
            .into_owned();

        Config {
            default_download_path: download_path,
            max_concurrent_tasks: 3,
            max_threads_per_task: 16,
            proxy_url: None,
            theme: String::from("system"),
            yt_dlp_version: None,
            split_audio_video: false,
            video_quality: String::from("best"),
            audio_quality: String::from("best"),
            browser_cookie: None,    // 新增默认：不使用浏览器 Cookie
            include_metadata: false, // 新增默认：关闭元数据归档
        }
    }

    /// 更新配置并持久化到本地文件
    pub fn update(&mut self, new_config: Config) -> Result<(), String> {
        self.settings = new_config;
        let content = serde_json::to_string_pretty(&self.settings).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}