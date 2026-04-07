use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 初始化嗅探器逻辑 (基于 Tauri 2 Webview 与脚本注入)
pub async fn init_sniffer(url: String, app: AppHandle) -> Result<(), String> {
    let label = "sniffer_window";

    // 如果嗅探窗口已存在，先关闭它以避免冲突
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.close();
    }

    // 核心拦截脚本 (Initialization Script)
    let init_script = r#"
        (function() {
            console.log("[DL-Omni] 嗅探脚本已成功注入！正在监听网络请求...");

            // 工具函数：将相对路径 (如 index.m3u8) 转换为完整的绝对路径
            function getAbsoluteUrl(url) {
                try { return new URL(url, window.location.href).href; }
                catch(e) { return url; }
            }

            // 统一的消息发送函数
            function tryEmit(url, type, source) {
                if (!url) return;
                const absUrl = getAbsoluteUrl(url);
                console.log(`[DL-Omni] 捕获到媒体流 (${source}):`, absUrl);

                if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                    window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                        event: "sniffed_resource",
                        payload: {
                            url: absUrl,
                            type: type,
                            filename: '媒体流 (' + source + ')'
                        }
                    }).then(() => {
                        console.log("[DL-Omni] 成功发送至前端！");
                    }).catch(err => {
                        console.error("[DL-Omni] IPC 发送至前端失败:", err);
                    });
                } else {
                    console.warn("[DL-Omni] 拦截成功，但 Tauri IPC 未就绪！请确保 capabilities 中配置了 remote urls。");
                }
            }

            // 1. 拦截 Fetch API
            const originalFetch = window.fetch;
            window.fetch = async function(...args) {
                const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');
                if (reqUrl && (reqUrl.includes('.m3u8') || reqUrl.includes('.mp4'))) {
                    tryEmit(reqUrl, reqUrl.includes('.m3u8') ? 'm3u8' : 'mp4', 'Fetch');
                }
                return originalFetch.apply(this, args);
            };

            // 2. 拦截 XMLHttpRequest
            const originalXhrOpen = XMLHttpRequest.prototype.open;
            XMLHttpRequest.prototype.open = function(method, reqUrl, ...rest) {
                if (typeof reqUrl === 'string' && (reqUrl.includes('.m3u8') || reqUrl.includes('.mp4'))) {
                    tryEmit(reqUrl, reqUrl.includes('.m3u8') ? 'm3u8' : 'mp4', 'XHR');
                }
                return originalXhrOpen.call(this, method, reqUrl, ...rest);
            };

            // 3. 兜底拦截：有些播放器直接向 <video src="..."> 赋值
            const originalVideoSrc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
            if (originalVideoSrc) {
                Object.defineProperty(HTMLMediaElement.prototype, 'src', {
                    get: function() { return originalVideoSrc.get.call(this); },
                    set: function(val) {
                        if (typeof val === 'string' && (val.includes('.m3u8') || val.includes('.mp4'))) {
                            tryEmit(val, val.includes('.m3u8') ? 'm3u8' : 'mp4', 'Video Src');
                        }
                        return originalVideoSrc.set.call(this, val);
                    }
                });
            }
        })();
    "#;

    // 动态创建独立的 Webview 窗口并注入拦截脚本
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url.parse().unwrap()))
        .title("DL-Omni - 资源嗅探器")
        .inner_size(1024.0, 768.0)
        .initialization_script(init_script)
        .build()
        .map_err(|e| format!("无法创建嗅探窗口: {}", e))?;

    Ok(())
}

/// 停止嗅探并销毁窗口
pub async fn stop_sniffer(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("sniffer_window") {
        win.close().map_err(|e| format!("关闭嗅探窗口失败: {}", e))?;
    }
    Ok(())
}