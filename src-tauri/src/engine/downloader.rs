use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::SeekFrom;
use tokio::io::{AsyncWriteExt, AsyncSeekExt};
use reqwest::Client;
use tauri::AppHandle;
use crate::models::{MediaInfo, Task, TaskStatus};
use crate::state::{AppState, TaskProgressUpdate};
use crate::utils;

// 伪装浏览器 UA，防止被 Github AWS S3 或 Cloudflare 等直接 403 阻截
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// 在解析阶段快速拉取直链信息并伪装成 MediaInfo 对象
pub async fn get_direct_link_info(url: &str) -> Result<MediaInfo, String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // 尝试发一个极小带宽的 GET 请求取代 HEAD，测试连通性
    let _ = client.get(url).header("Range", "bytes=0-0").send().await;

    let filename = utils::extract_filename_from_url(url);

    Ok(MediaInfo {
        id: "direct_link".to_string(),
        title: filename,
        duration: 0.0,
        thumbnail: "".to_string(),
        formats: vec![],
        playlist_entries: None,
    })
}

/// 针对直链或分片流的原生多线程/单线程流下载引擎
pub async fn download_native(_app: AppHandle, state: AppState, task: &Task) -> Result<u64, String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let mut total_size = 0;

    // 方案 1: 尝试 HEAD 请求获取体积
    if let Ok(res) = client.head(&task.url).send().await {
        total_size = res.content_length().unwrap_or(0);
    }

    // 方案 2: 若 HEAD 失败 (如 S3 限制)，尝试用 GET 拉取第一字节分析 Content-Range
    if total_size == 0 {
        if let Ok(res) = client.get(&task.url).header("Range", "bytes=0-0").send().await {
            if let Some(cr) = res.headers().get(reqwest::header::CONTENT_RANGE) {
                if let Ok(s) = cr.to_str() {
                    if let Some(total) = s.split('/').last() {
                        total_size = total.parse().unwrap_or(0);
                    }
                }
            }
            if total_size == 0 {
                total_size = res.content_length().unwrap_or(0);
            }
        }
    }

    // 若依然无法获取体积，则认为是不支持断点及多线程的服务器，触发流式单线程降级
    let is_stream_fallback = total_size == 0;

    // 读取全局配置
    let (save_dir, mut threads) = {
        let config = state.config.lock().await;
        (
            config.settings.default_download_path.clone(),
            config.settings.max_threads_per_task as u64
        )
    };

    if is_stream_fallback || total_size < 1024 * 1024 * 5 {
        threads = 1; // 体积过小或未知体积，强行单线程
    }

    std::fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;

    let filename = if task.title.is_empty() || task.title == "unknown_file" {
        utils::extract_filename_from_url(&task.url)
    } else {
        task.title.clone()
    };
    let file_path = std::path::Path::new(&save_dir).join(&filename);

    // 预分配空文件占位
    {
        let file = std::fs::File::create(&file_path).map_err(|e| e.to_string())?;
        if !is_stream_fallback {
            file.set_len(total_size).map_err(|e| e.to_string())?;
        }
    }

    let downloaded = Arc::new(AtomicU64::new(0));
    let mut handles = vec![];
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(u64, bytes::Bytes)>(threads as usize * 4);

    let writer_path = file_path.clone();
    let writer_handle = tokio::spawn(async move {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&writer_path)
            .await
            .unwrap();
        
        while let Some((offset, chunk)) = rx.recv().await {
            if file.seek(SeekFrom::Start(offset)).await.is_ok() {
                let _ = file.write_all(&chunk).await;
            }
        }
    });

    // 进度汇报定时器任务
    let reporter_total = total_size;
    let state_clone = state.clone();
    let task_id = task.id.clone();
    let downloaded_clone = downloaded.clone();
    
    let reporter_handle = tokio::spawn(async move {
        let mut last_bytes = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let current_bytes = downloaded_clone.load(Ordering::Relaxed);
            
            let speed = (current_bytes.saturating_sub(last_bytes)) as f64 * 2.0;
            let mut eta = 0;
            if reporter_total > 0 && speed > 0.0 {
                eta = (reporter_total.saturating_sub(current_bytes) as f64 / speed) as u64;
            }

            let mut buffer = state_clone.progress_buffer.lock().await;
            buffer.push(TaskProgressUpdate {
                id: task_id.clone(),
                downloaded_bytes: current_bytes,
                total_bytes: reporter_total,
                speed,
                eta,
                status: TaskStatus::Downloading,
            });

            last_bytes = current_bytes;
            if reporter_total > 0 && current_bytes >= reporter_total {
                break;
            }
        }
    });

    if is_stream_fallback {
        // 单线程死磕模式 (无需指定 Range，直接强行流式落盘)
        let url = task.url.clone();
        let tx = tx.clone();
        let downloaded = downloaded.clone();
        let client = client.clone();

        handles.push(tokio::spawn(async move {
            if let Ok(mut res) = client.get(&url).send().await {
                let mut current_offset = 0;
                while let Ok(Some(chunk)) = res.chunk().await {
                    let len = chunk.len() as u64;
                    if tx.send((current_offset, chunk)).await.is_err() {
                        break;
                    }
                    current_offset += len;
                    downloaded.fetch_add(len, Ordering::Relaxed);
                }
            }
        }));
    } else {
        // 多线程并发分片模式
        let chunk_size = total_size / threads;
        for i in 0..threads {
            let start = i * chunk_size;
            let end = if i == threads - 1 { total_size - 1 } else { (i + 1) * chunk_size - 1 };
            
            let url = task.url.clone();
            let tx = tx.clone();
            let downloaded = downloaded.clone();
            let client = client.clone();

            handles.push(tokio::spawn(async move {
                if let Ok(mut res) = client.get(&url).header("Range", format!("bytes={}-{}", start, end)).send().await {
                    let mut current_offset = start;
                    while let Ok(Some(chunk)) = res.chunk().await {
                        let len = chunk.len() as u64;
                        if tx.send((current_offset, chunk)).await.is_err() {
                            break;
                        }
                        current_offset += len;
                        downloaded.fetch_add(len, Ordering::Relaxed);
                    }
                }
            }));
        }
    }

    drop(tx);

    for handle in handles {
        let _ = handle.await;
    }
    let _ = writer_handle.await;
    reporter_handle.abort();

    let final_size = if is_stream_fallback {
        downloaded.load(Ordering::Relaxed)
    } else {
        total_size
    };

    if final_size == 0 {
        return Err("下载失败: 链接已失效、或者服务器拒绝连接 (403 Forbidden)".into());
    }

    Ok(final_size)
}