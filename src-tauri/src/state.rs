use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use crate::database::Db;
use crate::config::ConfigManager;

/// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Db>>,
    pub config: Arc<Mutex<ConfigManager>>,
    // 追踪正在运行的异步任务，便于随时 Abort
    pub active_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    // 进度缓冲池：所有子线程将进度写入此池中，由 Ticker 定时消费
    pub progress_buffer: Arc<Mutex<Vec<TaskProgressUpdate>>>,
}

/// 仅用于更新进度的轻量级载荷
#[derive(Clone, serde::Serialize)]
pub struct TaskProgressUpdate {
    pub id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub eta: u64,
    pub status: crate::models::TaskStatus,
}

/// 启动全局进度聚合节流器 (Ticker)
pub fn start_progress_ticker(app: AppHandle) {
    // 将 tokio::spawn 替换为 tauri::async_runtime::spawn
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
        loop {
            interval.tick().await;

            // 获取 AppState
            if let Some(state) = app.state::<AppState>().into() {
                let mut buffer = state.progress_buffer.lock().await;
                
                if !buffer.is_empty() {
                    // 将聚合后的进度批量发送给前端
                    if let Err(e) = app.emit("batch_progress_update", buffer.clone()) {
                        eprintln!("Failed to emit progress update: {}", e);
                    }
                    // 清空缓冲池，等待下一轮收集
                    buffer.clear();
                }
            }
        }
    });
}