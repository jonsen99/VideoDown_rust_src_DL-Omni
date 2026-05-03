/**
 * DL-Omni 嗅探核心引擎
 * 负责底层 Hook 并集成 PerformanceObserver 以实现全量网络监控
 */
(function() {
    console.log("[DL-Omni] 核心引擎加载，初始化全量监控网络面板...");

    const emittedUrls = new Set();

    function syncCookieToBackend() {
        if (document.cookie && window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                event: "sniffed_cookie",
                payload: {
                    domain: window.location.hostname,
                    cookie: document.cookie
                }
            }).catch(() => {});
        }
    }

    window.addEventListener('load', syncCookieToBackend);
    setInterval(syncCookieToBackend, 5000);

    function getPageMetadata() {
        try {
            const ogTitle = document.querySelector('meta[property="og:title"]');
            let title = ogTitle ? ogTitle.getAttribute('content') : document.title;

            if (title) {
                title = title.replace(/\s*is shared on PikPak.*$/i, '');
                title = title.replace(/\s*-\s*阿里云盘分享.*$/i, '');
                title = title.replace(/\s*-\s*夸克网盘分享.*$/i, '');
            }

            return title ? title.trim() : '未知网页';
        } catch (e) {
            return '未知网页';
        }
    }

    function extractFileInfo(url) {
        try {
            const parsed = new URL(url);
            const path = parsed.pathname;
            const filename = path.split('/').pop() || '';
            const extMatch = filename.match(/\.([a-zA-Z0-9]+)$/);
            return {
                original_name: filename || 'unknown',
                ext: extMatch ? extMatch[1].toLowerCase() : 'unknown'
            };
        } catch(e) {
            return { original_name: 'unknown', ext: 'unknown' };
        }
    }

    function getAbsoluteUrl(url) {
        try {
            return new URL(url, window.location.href).href;
        } catch(e) {
            return url;
        }
    }

    function determineCategory(ext, contentType) {
        const imageExts = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'webp'];
        const mediaExts = ['mp4', 'm3u8', 'ts', 'flv', 'mkv', 'webm', 'mov', 'mp3', 'm4a', 'wav'];
        const scriptExts = ['js', 'css', 'woff', 'woff2', 'ttf'];

        if (imageExts.includes(ext) || (contentType && contentType.includes('image/'))) return 'image';
        if (mediaExts.includes(ext) || (contentType && (contentType.includes('video/') || contentType.includes('audio/')))) return 'media';
        if (scriptExts.includes(ext) || (contentType && (contentType.includes('javascript') || contentType.includes('css')))) return 'script';
        if (contentType && (contentType.includes('json') || contentType.includes('xml'))) return 'xhr/fetch';

        return 'other';
    }

    window.__DL_OMNI_CORE__ = {
        tryEmit: function(url, type, source, mimeType = null, extraOptions = {}) {
            if (!url || typeof url !== 'string' || !url.startsWith('http')) return;

            try {
                const absUrl = getAbsoluteUrl(url);

                if (emittedUrls.has(absUrl)) return;
                emittedUrls.add(absUrl);

                let fileInfo = extractFileInfo(absUrl);

                if (mimeType && fileInfo.ext === 'unknown') {
                    const pureMime = mimeType.split(';')[0].trim().toLowerCase();
                    const mimeToExt = {
                        'video/mp4': 'mp4',
                        'video/x-flv': 'flv',
                        'video/x-matroska': 'mkv',
                        'video/webm': 'webm',
                        'video/quicktime': 'mov',
                        'audio/mpeg': 'mp3',
                        'audio/mp4': 'm4a',
                        'application/x-mpegurl': 'm3u8',
                        'application/vnd.apple.mpegurl': 'm3u8',
                        'application/dash+xml': 'mpd'
                    };
                    if (mimeToExt[pureMime]) {
                        fileInfo.ext = mimeToExt[pureMime];
                    }
                }

                if (extraOptions.ext) {
                    fileInfo.ext = extraOptions.ext;
                }

                const category = extraOptions.category || determineCategory(fileInfo.ext, mimeType);

                const headers = {
                    "Referer": window.location.href,
                    "User-Agent": navigator.userAgent,
                    "Cookie": document.cookie || ""
                };

                if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                    window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                        event: "sniffed_resource",
                        payload: {
                            url: absUrl,
                            type: type,
                            filename: `嗅探资源 [${source}]`,
                            page_title: getPageMetadata(),
                            original_name: fileInfo.original_name,
                            ext: fileInfo.ext,
                            headers: headers,
                            category: category,
                            is_highlighted: extraOptions.is_highlighted || false,
                            method: extraOptions.method || 'GET',
                            size: extraOptions.size || 0
                        }
                    }).catch(err => console.error("[DL-Omni] IPC 失败:", err));
                }
            } catch(e) {}
        }
    };

    function observeStaticResources() {
        if (typeof PerformanceObserver === 'undefined') return;

        const observer = new PerformanceObserver((list) => {
            const entries = list.getEntries();
            for (const entry of entries) {
                const url = entry.name;
                const type = entry.initiatorType;

                if (['xmlhttprequest', 'fetch'].includes(type)) continue;

                if (['img', 'css', 'script', 'link'].includes(type)) {
                    window.__DL_OMNI_CORE__.tryEmit(url, 'network', `Performance API (${type})`, null, {
                        method: 'GET',
                        size: entry.transferSize || 0
                    });
                }
            }
        });

        observer.observe({ entryTypes: ['resource'] });

        const initialResources = performance.getEntriesByType('resource');
        initialResources.forEach(entry => {
            const url = entry.name;
            const type = entry.initiatorType;
            if (!['xmlhttprequest', 'fetch'].includes(type) && ['img', 'css', 'script', 'link'].includes(type)) {
                window.__DL_OMNI_CORE__.tryEmit(url, 'network', `Performance API (${type})`, null, {
                    method: 'GET',
                    size: entry.transferSize || 0
                });
            }
        });
    }

    function getActiveAdapters(url) {
        return (window.__DL_OMNI_ADAPTERS__ || []).filter(adapter => adapter.match(url));
    }

    async function inspectResponse(reqUrl, method, cloneRes, source) {
        try {
            const currentUrl = window.location.href;
            const adapters = getActiveAdapters(currentUrl);
            const contentType = cloneRes.headers.get('content-type') || '';
            const size = parseInt(cloneRes.headers.get('content-length') || '0');

            let isProcessed = false;

            for (const adapter of adapters) {
                const hMatch = adapter.heuristicMatch(reqUrl);
                if (hMatch) {
                    const targetUrl = typeof hMatch === 'string' ? hMatch : (hMatch.url || reqUrl);
                    const extraOptions = typeof hMatch === 'object' ? hMatch : {
                        is_highlighted: true,
                        category: hMatch.category || 'media',
                        method: method,
                        ext: hMatch.ext,
                        size: size
                    };
                    window.__DL_OMNI_CORE__.tryEmit(targetUrl, 'media', `${source} - ${adapter.name} 启发式`, contentType, extraOptions);
                    isProcessed = true;
                    break;
                }
            }

            if (!isProcessed && (contentType.includes('application/json') || contentType.includes('text/') || contentType.includes('javascript'))) {
                const text = await cloneRes.text();
                if (text) {
                    for (const adapter of adapters) {
                        const result = adapter.interceptResponse(reqUrl, contentType, text);
                        if (result) {
                            const targetUrl = typeof result === 'string' ? result : (result.url || reqUrl);
                            const extraOptions = typeof result === 'object' ? result : {
                                is_highlighted: true,
                                category: result.category || 'xhr/fetch',
                                method: method,
                                ext: result.ext,
                                size: size
                            };
                            window.__DL_OMNI_CORE__.tryEmit(targetUrl, 'api', `${source} - ${adapter.name} 接口解析`, contentType, extraOptions);
                            isProcessed = true;
                            break;
                        }
                    }
                }
            }

            if (!isProcessed) {
                window.__DL_OMNI_CORE__.tryEmit(reqUrl, 'network', source, contentType, {
                    method: method,
                    category: determineCategory(extractFileInfo(reqUrl).ext, contentType),
                    size: size
                });
            }

        } catch(e) {}
    }

    const originalVideoSrc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
    if (originalVideoSrc) {
        Object.defineProperty(HTMLMediaElement.prototype, 'src', {
            set: function(value) {
                window.__DL_OMNI_CORE__.tryEmit(value, 'video', 'DOM Hook (src)', null, { category: 'media' });
                return originalVideoSrc.set.call(this, value);
            },
            get: function() {
                return originalVideoSrc.get.call(this);
            }
        });
    }

    const originalFetch = window.fetch;
    window.fetch = async function(...args) {
        const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');
        const method = (args[1] && args[1].method) ? args[1].method.toUpperCase() : 'GET';

        try {
            const response = await originalFetch.apply(this, args);
            inspectResponse(reqUrl, method, response.clone(), 'Fetch');
            return response;
        } catch (e) {
            window.__DL_OMNI_CORE__.tryEmit(reqUrl, 'network', 'Fetch (Failed)', null, { method: method });
            throw e;
        }
    };

    const originalXhrOpen = XMLHttpRequest.prototype.open;
    const originalXhrSend = XMLHttpRequest.prototype.send;

    XMLHttpRequest.prototype.open = function(method, url, ...rest) {
        this._reqUrl = url;
        this._reqMethod = method.toUpperCase();
        return originalXhrOpen.call(this, method, url, ...rest);
    };

    XMLHttpRequest.prototype.send = function(...args) {
        this.addEventListener('load', function() {
            try {
                const contentType = this.getResponseHeader('content-type') || '';
                const fakeRes = {
                    headers: new Headers({ 'content-type': contentType, 'content-length': this.getResponseHeader('content-length') || '0' }),
                    text: async () => {
                        if (this.responseType === '' || this.responseType === 'text') {
                            return this.responseText;
                        } else if (this.responseType === 'json') {
                            return typeof this.response === 'object' ? JSON.stringify(this.response) : this.response;
                        }
                        return "";
                    }
                };
                inspectResponse(this._reqUrl, this._reqMethod, fakeRes, 'XHR');
            } catch(e) {}
        });
        return originalXhrSend.apply(this, args);
    };

    observeStaticResources();

    // 防止链接在新窗口打开 (保持在嗅探器沙盒内)
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