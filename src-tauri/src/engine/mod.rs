pub mod ytdlp;
pub mod downloader;
pub mod updater;
pub mod sniffer;

use tauri::AppHandle;
use crate::models::{Task, TaskStatus};
use crate::state::{AppState, TaskProgressUpdate};

/// 核心调度器：目前强制全局路由至 yt-dlp 引擎
pub async fn dispatch_task(app: AppHandle, state: AppState, mut task: Task) -> Result<(), String> {
    let task_id = task.id.clone();

    // 标记任务为正在下载并更新数据库
    task.status = TaskStatus::Downloading;
    {
        let db = state.db.lock().await;
        let _ = db.update_status(&task_id, TaskStatus::Downloading);
    }

    let state_clone = state.clone();
    let app_clone = app.clone();

    // 创建一个 Tokio 异步任务
    let handle = tokio::spawn(async move {
        // --- 强制所有任务（包括嗅探出的直链和 m3u8）都交给 yt-dlp 处理 ---

        /* 暂时注释掉原生多线程下载器的路由逻辑
        let result = if is_direct_link(&task.url) {
            downloader::download_native(app_clone, state_clone.clone(), &task).await
        } else {
            ytdlp::download_via_ytdlp(app_clone, state_clone.clone(), &task).await
        };
        */

        // 全局使用 yt-dlp
        let result = ytdlp::download_via_ytdlp(app_clone, state_clone.clone(), &task).await;

        // 处理下载结果
        let (final_status, final_bytes) = match result {
            Ok(size) => {
                let _ = state_clone.db.lock().await.update_task_finish(&task.id, TaskStatus::Completed, size);
                (TaskStatus::Completed, size)
            },
            Err(e) => {
                eprintln!("Task {} failed: {}", task.id, e);
                // 将错误信息推送到前端并更新数据库
                let _ = state_clone.db.lock().await.update_status(&task.id, TaskStatus::Error);
                (TaskStatus::Error, task.total_bytes)
            }
        };

        // 清理 active_tasks 集合
        state_clone.active_tasks.lock().await.remove(&task.id);

        // 触发最终状态的进度聚合推送
        let mut buffer = state_clone.progress_buffer.lock().await;
        buffer.push(TaskProgressUpdate {
            id: task.id.clone(),
            downloaded_bytes: final_bytes,
            total_bytes: final_bytes,
            speed: 0.0,
            eta: 0,
            status: final_status,
        });
    });

    // 将任务句柄存入全局状态，以便随时可以 abort（暂停/取消）
    state.active_tasks.lock().await.insert(task_id, handle);

    Ok(())
}

// 暂且保留注释，后续开发原生下载器时再启用，避免编译时报 dead_code 警告
// /// 粗略判定是否为直链文件，用于决定使用的下载轨道
// fn is_direct_link(url: &str) -> bool {
//     url.ends_with(".mp4") || url.ends_with(".m3u8") || url.ends_with(".zip")
// }