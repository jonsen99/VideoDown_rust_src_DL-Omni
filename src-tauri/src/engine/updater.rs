use reqwest::Client;
use serde::Deserialize;
use std::fs;
use tauri::{AppHandle, Manager};
use tokio::process::Command;
use crate::state::AppState;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

// 辅助方法：创建携带代理的 HTTP 客户端
async fn create_client(app: &AppHandle) -> Result<Client, String> {
    let proxy_url = if let Some(state) = app.try_state::<AppState>() {
        let config = state.config.lock().await;
        config.settings.proxy_url.clone()
    } else {
        String::new()
    };

    let mut builder = Client::builder().user_agent("dl-omni-updater");

    if !proxy_url.trim().is_empty() {
        if let Ok(p) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(p);
        }
    }

    builder.build().map_err(|e| format!("构建 HTTP 客户端失败: {}", e))
}

/// 确保二进制文件存在，否则触发初次静默下载
pub fn ensure_binary_exists(app: AppHandle) {
    let bin_dir = crate::utils::get_binary_dir(&app);
    let bin_path = bin_dir.join(crate::utils::get_ytdlp_filename());

    if !bin_path.exists() {
        tauri::async_runtime::spawn(async move {
            let _ = check_and_update(app).await;
        });
    }
}

pub async fn check_and_update(app: AppHandle) -> Result<(bool, String), String> {
    let bin_dir = crate::utils::get_binary_dir(&app);
    let target_filename = crate::utils::get_ytdlp_filename();
    let final_path = bin_dir.join(&target_filename);

    let mut local_version = String::new();
    if final_path.exists() {
        let mut cmd = Command::new(&final_path);
        cmd.arg("--version");

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                local_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }

    let client = create_client(&app).await?;

    let url = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
    let response = client.get(url)
        .send().await.map_err(|e| format!("网络连接失败: {}", e))?;

    // 核心修复：先提取 status，因为 response.text() 会 consume (move) response 本身
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("请求 GitHub API 失败 (HTTP {}): {}", status, text));
    }

    let text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    let release: GithubRelease = serde_json::from_str(&text)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    let remote_version = release.tag_name.clone();

    if !local_version.is_empty() && local_version == remote_version {
        return Ok((false, remote_version));
    }

    let asset = release.assets.into_iter()
        .find(|a| a.name == target_filename)
        .ok_or("No matching binary found in the latest release")?;

    fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    let tmp_path = bin_dir.join(format!("{}.tmp", target_filename));

    let response = client.get(&asset.browser_download_url)
        .send().await.map_err(|e| e.to_string())?;

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&tmp_path, bytes).map_err(|e| e.to_string())?;

    fs::rename(&tmp_path, &final_path).map_err(|e| format!("Failed to swap binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&final_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&final_path, perms).map_err(|e| e.to_string())?;
    }

    Ok((true, remote_version))
}

pub fn ensure_ffmpeg_exists(app: AppHandle) {
    let bin_dir = crate::utils::get_binary_dir(&app);
    let bin_path = bin_dir.join(crate::utils::get_ffmpeg_filename());

    if !bin_path.exists() {
        tauri::async_runtime::spawn(async move {
            let _ = check_and_update_ffmpeg(app).await;
        });
    }
}

pub async fn check_and_update_ffmpeg(app: AppHandle) -> Result<String, String> {
    let client = create_client(&app).await?;

    let url = "https://api.github.com/repos/eugeneware/ffmpeg-static/releases/latest";
    let response = client.get(url)
        .send().await.map_err(|e| format!("网络连接失败: {}", e))?;

    // 核心修复：同上，提取 status 防止被 move
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("请求 GitHub API 失败 (HTTP {}): {}", status, text));
    }

    let text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    let release: GithubRelease = serde_json::from_str(&text)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    let target_asset_name = crate::utils::get_ffmpeg_asset_name();

    let asset = release.assets.into_iter()
        .find(|a| a.name.contains(target_asset_name) && !a.name.ends_with(".zip") && !a.name.ends_with(".gz"))
        .ok_or(format!("No matching raw ffmpeg binary '{}' found. (Please consider using Tauri Sidecar for ffmpeg)", target_asset_name))?;

    let bin_dir = crate::utils::get_binary_dir(&app);
    fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    let target_filename = crate::utils::get_ffmpeg_filename();
    let tmp_path = bin_dir.join(format!("{}.tmp", target_filename));
    let final_path = bin_dir.join(&target_filename);

    let response = client.get(&asset.browser_download_url)
        .send().await.map_err(|e| e.to_string())?;

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&tmp_path, bytes).map_err(|e| e.to_string())?;

    fs::rename(&tmp_path, &final_path).map_err(|e| format!("Failed to swap binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&final_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&final_path, perms).map_err(|e| e.to_string())?;
    }

    Ok(release.tag_name)
}

/// 将打包内置的二进制引擎拷贝到工作目录（用于离线完整版）
pub fn release_bundled_binaries(app: &AppHandle) {
    let bin_dir = crate::utils::get_binary_dir(app);
    let _ = fs::create_dir_all(&bin_dir);

    // 获取 Tauri 打包的资源目录
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled_bin_dir = resource_dir.join("binaries");

        let ytdlp_name = crate::utils::get_ytdlp_filename();
        let ffmpeg_name = crate::utils::get_ffmpeg_filename();

        let target_ytdlp = bin_dir.join(ytdlp_name);
        let target_ffmpeg = bin_dir.join(ffmpeg_name);

        let bundled_ytdlp = bundled_bin_dir.join(ytdlp_name);
        let bundled_ffmpeg = bundled_bin_dir.join(ffmpeg_name);

        // 如果工作目录没有 yt-dlp，且打包资源里有，则拷贝
        if !target_ytdlp.exists() && bundled_ytdlp.exists() {
            tracing::info!("检测到内置 yt-dlp，正在释放...");
            let _ = fs::copy(&bundled_ytdlp, &target_ytdlp);
            set_executable_permission(&target_ytdlp);
        }

        // 如果工作目录没有 ffmpeg，且打包资源里有，则拷贝
        if !target_ffmpeg.exists() && bundled_ffmpeg.exists() {
            tracing::info!("检测到内置 ffmpeg，正在释放...");
            let _ = fs::copy(&bundled_ffmpeg, &target_ffmpeg);
            set_executable_permission(&target_ffmpeg);
        }
    }
}

/// 辅助函数：为 Unix 系统设置可执行权限 (Windows 下为空操作)
pub fn set_executable_permission(_path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(_path, perms);
        }
    }
}