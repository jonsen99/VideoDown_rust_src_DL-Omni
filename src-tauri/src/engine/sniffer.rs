use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 初始化高级嗅探器逻辑 (基于 Tauri 2 Webview 与猫抓级多层级脚本注入)
pub async fn init_sniffer(url: String, app: AppHandle) -> Result<(), String> {
    let label = "sniffer_window";

    if let Some(win) = app.get_webview_window(label) {
        let _ = win.close();
    }

    // 核心拦截脚本：实现了 DOM 劫持、更稳健的启发式匹配、MIME 拦截与安全的 JSON 解析
    let init_script = r#"
        (function() {
            console.log("[DL-Omni] 猫抓级高级嗅探脚本已注入，开启底层多路侦听...");
            
            // 去重池
            const emittedUrls = new Set();

            function getAbsoluteUrl(url) {
                try {
                    // 自动补全相对路径，并返回包含所有参数的完整标准 URL (href)
                    return new URL(url, window.location.href).href;
                } catch(e) {
                    return url;
                }
            }

            function tryEmit(url, type, source) {
                if (!url || typeof url !== 'string' || !url.startsWith('http')) return;

                try {
                    const absUrl = getAbsoluteUrl(url);

                    // 【精准去重】严格要求包含参数在内的完整链接完全一致，才会被判定为重复并拦截
                    if (emittedUrls.has(absUrl)) return;
                    emittedUrls.add(absUrl);

                    console.log(`[DL-Omni] 捕获媒体流 (${source}):`, absUrl);

                    const headers = {
                        "Referer": window.location.href,
                        "User-Agent": navigator.userAgent,
                        "Cookie": document.cookie || ""
                    };

                    // 修正 IPC 嵌套结构，恢复 Tauri 2 原生的扁平 Payload 结构
                    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                        window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                            event: "sniffed_resource",
                            payload: {
                                url: absUrl,
                                type: type,
                                filename: `嗅探资源 [${source}]`,
                                headers: headers
                            }
                        }).catch(err => console.error("[DL-Omni] IPC 失败:", err));
                    }
                } catch(e) {}
            }

            // ==========================================
            // 阶段一：DOM 原型链劫持
            // ==========================================
            const originalVideoSrc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
            if (originalVideoSrc) {
                Object.defineProperty(HTMLMediaElement.prototype, 'src', {
                    set: function(value) {
                        tryEmit(value, 'video', 'DOM Hook (src)');
                        return originalVideoSrc.set.call(this, value);
                    },
                    get: function() {
                        return originalVideoSrc.get.call(this);
                    }
                });
            }

            const originalSetAttribute = Element.prototype.setAttribute;
            Element.prototype.setAttribute = function(name, value) {
                if (this instanceof HTMLMediaElement && name === 'src') {
                    tryEmit(value, 'video', 'DOM Hook (setAttribute)');
                }
                return originalSetAttribute.apply(this, arguments);
            };

            // ==========================================
            // 阶段二：健壮的启发式 URL 匹配
            // ==========================================
            function heuristicMatch(url) {
                const u = url.toLowerCase();
                // 针对 PikPak 增加 /download/ 路径的严格限制，避免误抓普通 API 接口
                if ((u.includes('mypikpak.com') || u.includes('pikpak.io') || u.includes('pikpak.net')) && u.includes('/download/') && (u.includes('sign=') || u.includes('signature='))) return 'PikPak';
                if ((u.includes('aliyundrive.net') || u.includes('alipan.com')) && (u.includes('signature=') || u.includes('auth_key='))) return 'AliYunPan';
                return null;
            }

            // ==========================================
            // 阶段三：网络请求响应体解析
            // ==========================================
            async function inspectResponse(url, cloneRes, source) {
                try {
                    const hMatch = heuristicMatch(url);
                    if (hMatch) {
                        tryEmit(url, 'video', `${source} - 启发式 (${hMatch})`);
                        return;
                    }

                    const contentType = cloneRes.headers.get('content-type') || '';

                    if (contentType.includes('video/') || contentType.includes('audio/') ||
                        contentType.includes('mpegurl') || contentType.includes('dash+xml') ||
                        contentType.includes('application/octet-stream')) {

                        if (contentType.includes('application/octet-stream')) {
                            if (url.includes('fid=') || url.includes('sign=') || url.includes('token=')) {
                                tryEmit(url, 'media (octet-stream)', `${source} MIME`);
                            }
                        } else {
                            tryEmit(url, contentType.split('/')[0] || 'media', `${source} MIME`);
                        }
                        return;
                    }

                    if (contentType.includes('application/json')) {
                        const text = await cloneRes.text();
                        if (!text) return;

                        try {
                            const data = JSON.parse(text);
                            // 【核心修复】修复 JSON 遍历，采用白名单+黑名单结合，防止误抓数组里的心跳接口
                            const findUrl = (obj) => {
                                if (!obj || typeof obj !== 'object') return;
                                for (const key in obj) {
                                    if (typeof obj[key] === 'string' && (obj[key].startsWith('http://') || obj[key].startsWith('https://'))) {
                                        const k = key.toLowerCase();
                                        const v = obj[key].toLowerCase();

                                        // 1. 如果 JSON 内的链接直接符合 PikPak / 阿里云盘 的规则，直接提取
                                        const nestedMatch = heuristicMatch(obj[key]);
                                        if (nestedMatch) {
                                            tryEmit(obj[key], 'video', `${source} - API 安全解析 (${nestedMatch})`);
                                            continue;
                                        }

                                        // 2. 黑名单过滤：排除明显的无效接口 (心跳、配置、日志、上报等)
                                        const isGarbage = v.includes('.health') || v.includes('config') || v.includes('/log') || v.includes('/report');
                                        if (isGarbage) continue;

                                        // 3. 白名单提取：包含明确视频后缀，或者键名明确指向播放地址
                                        const hasMediaExt = v.includes('.mp4') || v.includes('.m3u8') || v.includes('.flv') || v.includes('.mkv');
                                        const isPlayKey = k.includes('play') || k.includes('video') || k.includes('m3u8');

                                        if (hasMediaExt || isPlayKey) {
                                            tryEmit(obj[key], 'video', `${source} - API 安全解析`);
                                        }
                                    } else if (typeof obj[key] === 'object') {
                                        findUrl(obj[key]);
                                    }
                                }
                            };
                            findUrl(data);
                        } catch(jsonErr) {
                            // 正则兜底：应对非标 JSON 或某些特定平台的粗暴提取
                            const dyMatch = text.match(/"url_list"\s*:\s*\["([^"]+)"\]/);
                            if (dyMatch && dyMatch[1]) tryEmit(dyMatch[1].replace(/\\u0026/g, '&'), 'video', `${source} - 正则兜底`);
                        }
                    }
                } catch(e) {}
            }

            // ==========================================
            // 阶段四：底层 Fetch & XHR 劫持
            // ==========================================
            const originalFetch = window.fetch;
            window.fetch = async function(...args) {
                const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');

                const hMatch = heuristicMatch(reqUrl);
                if (hMatch) {
                    tryEmit(reqUrl, 'video', `Fetch - 启发式 (${hMatch})`);
                }

                const response = await originalFetch.apply(this, args);
                inspectResponse(reqUrl, response.clone(), 'Fetch');
                return response;
            };

            const originalXhrOpen = XMLHttpRequest.prototype.open;
            const originalXhrSend = XMLHttpRequest.prototype.send;

            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this._reqUrl = url;
                const hMatch = heuristicMatch(url);
                if (hMatch) {
                    tryEmit(url, 'video', `XHR - 启发式 (${hMatch})`);
                }
                return originalXhrOpen.call(this, method, url, ...rest);
            };

            XMLHttpRequest.prototype.send = function(...args) {
                this.addEventListener('load', function() {
                    try {
                        const contentType = this.getResponseHeader('content-type') || '';
                        const fakeRes = {
                            headers: new Headers({ 'content-type': contentType }),
                            text: async () => {
                                // 放开对 json 响应类型的读取，防止提取特征失败
                                if (this.responseType === '' || this.responseType === 'text') {
                                    return this.responseText;
                                } else if (this.responseType === 'json') {
                                    return typeof this.response === 'object' ? JSON.stringify(this.response) : this.response;
                                }
                                return "";
                            }
                        };
                        inspectResponse(this._reqUrl, fakeRes, 'XHR');
                    } catch(e) {}
                });
                return originalXhrSend.apply(this, args);
            };

            // ==========================================
            // 阶段五：防止链接在新窗口打开
            // ==========================================
            document.addEventListener('click', function(e) {
                let target = e.target;
                while (target && target.tagName !== 'A') {
                    target = target.parentNode;
                }
                if (target && target.tagName === 'A' && target.getAttribute('target') === '_blank') {
                    target.removeAttribute('target');
                }
            }, true);

            const originalOpen = window.open;
            window.open = function(url, target, features) {
                if (target === '_blank') target = '_self';
                return originalOpen.call(window, url, target, features);
            };

        })();
    "#;

    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url.parse().unwrap()))
        .title("DL-Omni - 资源嗅探器 (猫抓级多路引擎)")
        .inner_size(1100.0, 800.0)
        .initialization_script(init_script)
        .build()
        .map_err(|e| format!("无法创建嗅探窗口: {}", e))?;

    Ok(())
}

pub async fn stop_sniffer(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("sniffer_window") {
        win.close().map_err(|e| format!("关闭嗅探窗口失败: {}", e))?;
    }
    Ok(())
}