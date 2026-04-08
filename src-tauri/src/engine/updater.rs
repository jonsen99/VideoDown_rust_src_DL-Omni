use reqwest::Client;
use serde::Deserialize;
use std::fs;
use tauri::AppHandle;
use tokio::process::Command;

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

/// 确保二进制文件存在，否则触发初次静默下载
pub fn ensure_binary_exists(app: AppHandle) {
    let bin_dir = crate::utils::get_binary_dir(&app);
    let bin_path = bin_dir.join(crate::utils::get_ytdlp_filename());
    
    if !bin_path.exists() {
        // 如果文件不存在，放入后台 Tokio 任务进行初始化下载
        tauri::async_runtime::spawn(async move {
            let _ = check_and_update(app).await;
        });
    }
}

/// 检查 GitHub Release API 并静默更新 yt-dlp
/// 返回元组: (是否执行了下载更新, 最终最新版本号)
pub async fn check_and_update(app: AppHandle) -> Result<(bool, String), String> {
    let bin_dir = crate::utils::get_binary_dir(&app);
    let target_filename = crate::utils::get_ytdlp_filename();
    let final_path = bin_dir.join(&target_filename);

    // 1. 尝试获取本地当前真实版本号
    let mut local_version = String::new();
    if final_path.exists() {
        let mut cmd = Command::new(&final_path);
        cmd.arg("--version");
        
        // 隐藏 Windows 下可能弹出的黑框
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                local_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }

    // 2. 获取云端最新版本号
    let client = Client::builder()
        .user_agent("dl-omni-updater")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let url = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
    let release: GithubRelease = client.get(url)
        .send().await.map_err(|e| format!("Network error: {}", e))?
        .json().await.map_err(|e| format!("JSON parse error: {}", e))?;

    let remote_version = release.tag_name.clone();

    // 3. 版本比对：如果一致，则直接返回，跳过耗时的下载过程
    if !local_version.is_empty() && local_version == remote_version {
        return Ok((false, remote_version));
    }

    // 4. 版本不一致或本地文件缺失，开始真正下载新核心
    let asset = release.assets.into_iter()
        .find(|a| a.name == target_filename)
        .ok_or("No matching binary found in the latest release")?;

    fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    let tmp_path = bin_dir.join(format!("{}.tmp", target_filename));

    // 下载新版二进制文件到 .tmp 临时路径
    let response = client.get(&asset.browser_download_url)
        .send().await.map_err(|e| e.to_string())?;
    
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&tmp_path, bytes).map_err(|e| e.to_string())?;

    // TODO: 核心防错 - 在此处遍历系统进程，kill 掉所有名为 yt-dlp 的孤儿进程，防止因文件被占用导致重命名失败 (需引入 sysinfo 库)

    // 重命名替换旧文件 (Safe Swap)
    fs::rename(&tmp_path, &final_path).map_err(|e| format!("Failed to swap binary: {}", e))?;

    // Unix 平台需要赋予可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&final_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&final_path, perms).map_err(|e| e.to_string())?;
    }

    Ok((true, remote_version))
}

/// 确保 ffmpeg 二进制文件存在，否则触发初次静默下载
pub fn ensure_ffmpeg_exists(app: AppHandle) {
    let bin_dir = crate::utils::get_binary_dir(&app);
    let bin_path = bin_dir.join(crate::utils::get_ffmpeg_filename());
    
    if !bin_path.exists() {
        tauri::async_runtime::spawn(async move {
            let _ = check_and_update_ffmpeg(app).await;
        });
    }
}

/// 检查并下载轻量静态编译版 ffmpeg
pub async fn check_and_update_ffmpeg(app: AppHandle) -> Result<String, String> {
    let client = Client::builder()
        .user_agent("dl-omni-updater")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let url = "https://api.github.com/repos/eugeneware/ffmpeg-static/releases/latest";
    let release: GithubRelease = client.get(url)
        .send().await.map_err(|e| format!("Network error: {}", e))?
        .json().await.map_err(|e| format!("JSON parse error: {}", e))?;

    let target_asset_name = crate::utils::get_ffmpeg_asset_name();
    
    let asset = release.assets.into_iter()
        .find(|a| a.name == target_asset_name)
        .ok_or(format!("No matching ffmpeg binary '{}' found in the latest release", target_asset_name))?;

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