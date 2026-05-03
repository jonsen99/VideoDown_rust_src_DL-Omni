class GdStudioAdapter extends SnifferAdapter {
    constructor() {
        super('GdStudio');
    }

    match(currentUrl) {
        return currentUrl.includes('music.gdstudio.xyz') || currentUrl.includes('api.php');
    }

    heuristicMatch(reqUrl) {
        return null;
    }

    interceptResponse(reqUrl, contentType, responseData) {
        if (reqUrl.includes('api.php')) {
            try {
                let dataText = responseData;
                
                if (typeof responseData === 'string') {
                    const jsonpMatch = responseData.match(/^[a-zA-Z0-9_]+\s*\(([\s\S]*)\)\s*;?$/);
                    if (jsonpMatch && jsonpMatch[1]) {
                        dataText = jsonpMatch[1];
                    }
                }
                
                const data = typeof dataText === 'string' ? JSON.parse(dataText) : dataText;
                
                const isSearchResult = Array.isArray(data) && data.length > 0 && (data[0].url_id || data[0].lyric_id);

                if (isSearchResult) {
                    console.log("[DL-Omni] 捕获到 GdStudio 搜索结果:", data);

                    return {
                        url: reqUrl,
                        is_highlighted: true,
                        category: 'xhr/fetch',
                        ext: 'json'
                    };
                }
            } catch (e) {
            }
        }
        return null;
    }
}

window.__DL_OMNI_ADAPTERS__.push(new GdStudioAdapter());