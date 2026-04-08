use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 初始化高级嗅探器逻辑 (基于 Tauri 2 Webview 与猫抓级多层级脚本注入)
pub async fn init_sniffer(url: String, app: AppHandle) -> Result<(), String> {
    let label = "sniffer_window";

    if let Some(win) = app.get_webview_window(label) {
        let _ = win.close();
    }

    // 核心拦截脚本：实现了 DOM 劫持、启发式正则、MIME 拦截与 JSON API 特征提取
    let init_script = r#"
        (function() {
            console.log("[DL-Omni] 猫抓级高级嗅探脚本已注入，开启底层多路侦听...");
            
            // 去重池，避免同一链接高频触发 IPC
            const emittedUrls = new Set();

            function getAbsoluteUrl(url) {
                try { return new URL(url, window.location.href).href; }
                catch(e) { return url; }
            }

            // 统一的消息发送函数 (自动附带网页的 Referer, User-Agent 以及极度关键的 Cookie)
            function tryEmit(url, type, source) {
                if (!url || typeof url !== 'string' || !url.startsWith('http')) return;
                
                const absUrl = getAbsoluteUrl(url);
                if (emittedUrls.has(absUrl)) return;
                emittedUrls.add(absUrl);

                console.log(`[DL-Omni] 捕获媒体流 (${source}):`, absUrl);

                // 组装动态 Headers，完整克隆当前网页的鉴权环境
                const headers = {
                    "Referer": window.location.href,
                    "User-Agent": navigator.userAgent,
                    "Cookie": document.cookie || "" // 突破网盘直链鉴权的核心
                };

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
            }

            // ==========================================
            // 阶段一：DOM 原型链劫持 (DOM Hook)
            // 专门捕获那些不走网络 API，直接由 JS 赋值给原生标签的防盗链
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
            // 阶段二：启发式 URL 正则匹配 (Heuristic Regex)
            // 应对跨域不透明响应(Opaque)导致的 Content-Type 丢失
            // ==========================================
            function heuristicMatch(url) {
                const pikpakRegex = /dl-.*\.mypikpak\.com\/download\/.*fid=.*sign=/i;
                const pikpakDomainRegex = /dl\.pikpak\.io\/.*\/signature\/.*\/ttl\//i;
                const alipanRegex = /cn-.*\.aliyundrive\.net\/.*x-oss-signature=/i;
                
                if (pikpakRegex.test(url) || pikpakDomainRegex.test(url)) return 'PikPak API/直链';
                if (alipanRegex.test(url)) return 'AliYunPan 直链';
                
                return null;
            }

            // ==========================================
            // 阶段三：网络请求响应体解析 (MIME + JSON API)
            // ==========================================
            async function inspectResponse(url, cloneRes, source) {
                try {
                    // 1. 优先执行启发式 URL 正则匹配
                    const hMatch = heuristicMatch(url);
                    if (hMatch) {
                        tryEmit(url, 'video', `${source} - 启发式正则 (${hMatch})`);
                        return;
                    }

                    const contentType = cloneRes.headers.get('content-type') || '';
                    
                    // 2. 基于 MIME 类型拦截 (加入对通用二进制流的放宽策略)
                    if (contentType.includes('video/') || contentType.includes('audio/') || 
                        contentType.includes('mpegurl') || contentType.includes('dash+xml') ||
                        contentType.includes('application/octet-stream')) {
                        
                        // 对于 octet-stream，需要进一步约束防止抓到普通的系统文件
                        if (contentType.includes('application/octet-stream')) {
                            if (url.includes('fid=') || url.includes('sign=') || url.includes('token=')) {
                                tryEmit(url, 'media (octet-stream)', `${source} MIME`);
                            }
                        } else {
                            tryEmit(url, contentType.split('/')[0] || 'media', `${source} MIME`);
                        }
                        return;
                    }

                    // 3. JSON API 特征提取 (针对返回了真实直链的 JSON 报文)
                    if (contentType.includes('application/json')) {
                        const text = await cloneRes.text();
                        
                        // 抖音特征
                        const dyMatch = text.match(/"url_list"\s*:\s*\["([^"]+)"\]/);
                        if (dyMatch && dyMatch[1]) {
                            const cleanUrl = dyMatch[1].replace(/\\u0026/g, '&');
                            tryEmit(cleanUrl, 'video', `${source} - API解析 (Douyin)`);
                        }
                        
                        // PikPak API 特征提取
                        const ppMatch = text.match(/"web_content_link"\s*:\s*"([^"]+)"/);
                        if (ppMatch && ppMatch[1]) {
                            const cleanUrl = ppMatch[1].replace(/\\u0026/g, '&');
                            tryEmit(cleanUrl, 'video', `${source} - API解析 (PikPak)`);
                        }
                    }
                } catch(e) {
                    // 忽略跨域或读取流报错
                }
            }

            // ==========================================
            // 阶段四：底层 Fetch & XHR 劫持
            // ==========================================
            const originalFetch = window.fetch;
            window.fetch = async function(...args) {
                const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');
                
                // 预检：请求发出前先过一遍 URL 正则
                const hMatch = heuristicMatch(reqUrl);
                if (hMatch) {
                    tryEmit(reqUrl, 'video', `Fetch Request - 启发式 (${hMatch})`);
                }

                const response = await originalFetch.apply(this, args);
                // clone response 防破坏原网页读取
                inspectResponse(reqUrl, response.clone(), 'Fetch');
                return response;
            };

            const originalXhrOpen = XMLHttpRequest.prototype.open;
            const originalXhrSend = XMLHttpRequest.prototype.send;
            
            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this._reqUrl = url;
                // 预检
                const hMatch = heuristicMatch(url);
                if (hMatch) {
                    tryEmit(url, 'video', `XHR Request - 启发式 (${hMatch})`);
                }
                return originalXhrOpen.call(this, method, url, ...rest);
            };
            
            XMLHttpRequest.prototype.send = function(...args) {
                this.addEventListener('load', function() {
                    try {
                        const contentType = this.getResponseHeader('content-type') || '';
                        const fakeRes = {
                            headers: new Headers({ 'content-type': contentType }),
                            text: async () => this.responseText
                        };
                        inspectResponse(this._reqUrl, fakeRes, 'XHR');
                    } catch(e) {}
                });
                return originalXhrSend.apply(this, args);
            };

            // ==========================================
            // 阶段五：防止链接在新窗口打开 (强制当前窗口)
            // ==========================================
            document.addEventListener('click', function(e) {
                let target = e.target;
                while (target && target.tagName !== 'A') {
                    target = target.parentNode;
                }
                if (target && target.tagName === 'A') {
                    // 如果是 target="_blank"，移除它
                    if (target.getAttribute('target') === '_blank') {
                        console.log("[DL-Omni] 拦截到 target='_blank' 链接，已重定向至当前窗口");
                        target.removeAttribute('target');
                    }
                }
            }, true);

            // 劫持 window.open
            const originalOpen = window.open;
            window.open = function(url, target, features) {
                console.log("[DL-Omni] 拦截到 window.open 调用，已重定向至当前窗口");
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