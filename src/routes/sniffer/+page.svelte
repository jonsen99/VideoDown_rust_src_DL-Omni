<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { IPC } from '$lib/api/ipc';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import { goto } from '$app/navigation';
  import { formatUrl } from '$lib/utils/url';
  import type { SniffedResource } from '$lib/types';
  import { page } from '$app/stores';

  let url = $state('https://www.douyin.com/jingxuan');
  let capturedResources = $state<SniffedResource[]>([]);
  let showDrawer = $state(false);
  let isSniffing = $state(false);
  let unlisten: UnlistenFn | null = null;
  let unlistenClosed: UnlistenFn | null = null;

  let isPaused = $state(false); 
  let filterText = $state('');  
  let selectedCategory = $state('all'); 
  let selectedUrls = $state<Set<string>>(new Set());

  const categories = [
    { id: 'all', label: '全部' },
    { id: 'highlighted', label: '🌟 高亮解析' },
    { id: 'media', label: '视频/音频' },
    { id: 'xhr/fetch', label: 'Fetch/XHR' },
    { id: 'image', label: '图片' },
    { id: 'script', label: '脚本/样式' },
    { id: 'other', label: '其他' }
  ];

  let filteredResources = $derived(
    capturedResources.filter(res => {
      if (filterText) {
        const lowerFilter = filterText.toLowerCase();
        const title = taskStore.parseTemplate(res).toLowerCase();
        if (!title.includes(lowerFilter) && !res.url.toLowerCase().includes(lowerFilter)) {
          return false;
        }
      }

      if (selectedCategory === 'all') return true;
      if (selectedCategory === 'highlighted') return res.is_highlighted === true;

      return res.category === selectedCategory;
    })
  );

  let isAllSelected = $derived(
    filteredResources.length > 0 &&
    filteredResources.every(res => selectedUrls.has(res.url))
  );

  const shortcuts = [
    { title: '音乐检索', url: 'https://music.gdstudio.xyz/', desc: 'music.gdstudio.xyz', icon: 'M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3' },
    { title: '视频搜索', url: 'https://www.iyf.lv/', desc: 'www.iyf.lv', icon: 'M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z' },
    { title: '网盘搜索', url: 'https://cse.google.com/cse?cx=e7dbb37893b8e4dbf', desc: 'Google CSE 聚合', icon: 'M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z' }
  ];

  onMount(async () => {
    const queryUrl = $page.url.searchParams.get('url');
    if (queryUrl) {
      url = queryUrl;
    }

    unlisten = await IPC.listenSniffedResources((resource: SniffedResource) => {
      if (isPaused) return;

      const blacklist = configStore.settings.sniff_blacklist;
      if (blacklist && new RegExp(blacklist, 'i').test(resource.url)) {
        return;
      }

      if (!capturedResources.find(r => r.url === resource.url)) {
        capturedResources = [resource, ...capturedResources];

        if (capturedResources.length > 2000) {
            capturedResources = capturedResources.slice(0, 2000);
        }
      }
    });

    unlistenClosed = await IPC.listenSnifferClosed(() => {
      isSniffing = false;
    });
  });

  onDestroy(async () => {
    if (unlisten) unlisten();
    if (unlistenClosed) unlistenClosed();
    if (isSniffing) await IPC.stopSniffing();
  });

  async function start() {
    if (!url) return;
    url = formatUrl(url);

    isSniffing = true;
    isPaused = false;
    capturedResources = [];
    selectedUrls.clear();
    showDrawer = false;

    try {
      await IPC.startSniffing(url);
    } catch (e) {
      console.error("嗅探启动失败:", e);
      isSniffing = false;
    }
  }

  async function stop() {
    isSniffing = false;
    try { await IPC.stopSniffing(); } catch (e) { console.error("停止嗅探失败:", e); }
  }

  function handleDownload(resource: SniffedResource) {
    showDrawer = false;
    taskStore.submitSniffedTask(resource);
    goto('/');
  }

  function handleBatchDownload() {
    if (selectedUrls.size === 0) return;

    showDrawer = false;
    const resourcesToDownload = capturedResources.filter(r => selectedUrls.has(r.url));

    for (const res of resourcesToDownload) {
      taskStore.submitSniffedTask(res);
    }

    selectedUrls.clear();
    goto('/');
  }

  function toggleSelectAll() {
    if (isAllSelected) {
      filteredResources.forEach(res => selectedUrls.delete(res.url));
    } else {
      filteredResources.forEach(res => selectedUrls.add(res.url));
    }
    selectedUrls = new Set(selectedUrls);
  }

  function toggleSelection(resourceUrl: string) {
    if (selectedUrls.has(resourceUrl)) {
      selectedUrls.delete(resourceUrl);
    } else {
      selectedUrls.add(resourceUrl);
    }
    selectedUrls = new Set(selectedUrls);
  }

  function formatSize(bytes?: number) {
    if (!bytes) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function handleShortcutClick(targetUrl: string) {
    url = targetUrl;
    start();
  }
</script>

<div class="h-full flex flex-col relative bg-zinc-950">
  <header class="shrink-0 p-3 flex items-center space-x-2 border-b border-zinc-800/50 bg-zinc-900">
    <div class="flex-1 relative">
      <input
        type="text"
        bind:value={url}
        placeholder="输入流媒体网页地址 (回车开始嗅探，可直接输入 baidu.com)"
        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-sm text-zinc-200 outline-none focus:border-accent-blue transition-colors"
        onkeydown={(e) => e.key === 'Enter' && start()}
      />
    </div>
    {#if isSniffing}
      <button class="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-500 text-sm font-medium rounded-lg transition-colors" onclick={stop}>停止引擎</button>
    {:else}
      <button class="px-4 py-2 bg-accent-blue hover:bg-blue-600 text-white text-sm font-medium rounded-lg transition-colors" onclick={start}>开启高级嗅探窗</button>
    {/if}
  </header>

  <div class="flex-1 relative flex flex-col items-center justify-center p-6 overflow-y-auto">
    {#if !isSniffing}
      <div class="flex flex-col items-center justify-center mt-[-10vh]">
        <svg class="w-16 h-16 text-zinc-700 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
        <h3 class="text-lg font-medium text-zinc-300 mb-2">等待输入网页</h3>
        <p class="text-sm text-zinc-500 text-center max-w-sm mb-12">全量监控模式：集成 Performance API 捕获包含图片、样式在内的所有网络活动。</p>

        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 w-full max-w-2xl px-4">
          {#each shortcuts as item}
            <button class="group flex flex-col items-center p-5 bg-zinc-800/20 border border-zinc-800 rounded-xl hover:bg-zinc-800/50 transition-all" onclick={() => handleShortcutClick(item.url)}>
              <div class="w-12 h-12 rounded-full bg-zinc-800 flex items-center justify-center mb-3 group-hover:bg-accent-blue/10">
                <svg class="w-6 h-6 text-zinc-400 group-hover:text-accent-blue transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={item.icon} /></svg>
              </div>
              <span class="text-sm font-medium text-zinc-200">{item.title}</span>
            </button>
          {/each}
        </div>
      </div>
    {:else}
      <div class="relative w-24 h-24 mb-6 flex items-center justify-center">
        <div class="absolute inset-0 border-4 border-zinc-800 rounded-full"></div>
        <div class="absolute inset-0 border-4 border-accent-blue rounded-full border-t-transparent animate-spin"></div>
        <svg class="w-8 h-8 text-accent-blue" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
      </div>
      <h3 class="text-lg font-medium text-zinc-200 mb-2">底层拦截引擎运行中</h3>
      <p class="text-sm text-zinc-500 text-center">请在弹出的窗口中播放视频或发起网络请求...</p>
    {/if}
  </div>

  <button
    class="absolute right-8 bottom-8 w-14 h-14 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-full shadow-xl flex items-center justify-center z-40 group"
    onclick={() => showDrawer = !showDrawer}
    aria-label="查看网络面板"
    title="网络面板"
  >
    <svg class="w-6 h-6 text-zinc-300" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 002-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/></svg>
    {#if capturedResources.length > 0}
      <span class="absolute top-0 right-0 flex h-5 w-5 rounded-full bg-accent-blue items-center justify-center text-[10px] font-bold text-white">{capturedResources.length}</span>
    {/if}
  </button>

  {#if showDrawer}
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm z-40"
      role="button"
      tabindex="0"
      onclick={() => showDrawer = false}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') showDrawer = false; }}
      aria-label="关闭抽屉层"
    ></div>

    <div class="absolute inset-x-0 bottom-0 h-[38rem] bg-zinc-900 border-t border-zinc-700 shadow-2xl flex flex-col z-50">
      <div class="flex flex-col p-4 border-b border-zinc-800/80 space-y-3 bg-zinc-900 shrink-0">
        <div class="flex justify-between items-center">
          <h3 class="text-sm font-medium text-zinc-100 flex items-center">
            网络面板 <span class="ml-2 px-1.5 py-0.5 bg-zinc-800 rounded text-xs">{filteredResources.length}</span>
          </h3>
          <div class="flex items-center space-x-2">
            <button
              class="flex items-center space-x-1 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border {isPaused ? 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20 hover:bg-emerald-500/20' : 'bg-zinc-800 text-zinc-300 border-zinc-700 hover:bg-zinc-700'}"
              onclick={() => isPaused = !isPaused}
            >
              {#if isPaused}
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                <span>继续记录</span>
              {:else}
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                <span>暂停记录</span>
              {/if}
            </button>
            <button
              onclick={() => showDrawer = false}
              class="p-1.5 text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 rounded transition-colors"
              aria-label="关闭"
              title="关闭"
            >
              <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
            </button>
          </div>
        </div>

        <div class="flex space-x-1 bg-zinc-800/50 p-1 rounded-lg overflow-x-auto hide-scrollbar">
          {#each categories as cat}
            <button
              class="px-3 py-1.5 text-[11px] font-medium rounded-md whitespace-nowrap transition-colors {selectedCategory === cat.id ? 'bg-zinc-700 text-zinc-100 shadow-sm' : 'text-zinc-400 hover:text-zinc-200'}"
              onclick={() => selectedCategory = cat.id}
            >
              {cat.label}
            </button>
          {/each}
        </div>

        <div class="flex items-center space-x-3">
          <button
            class="flex items-center space-x-1.5 px-3 py-1.5 bg-zinc-800/50 hover:bg-zinc-800 border border-zinc-700/50 rounded-md text-xs text-zinc-300 transition-colors shrink-0"
            onclick={toggleSelectAll}
          >
            <div class="w-3.5 h-3.5 rounded border flex items-center justify-center transition-colors {isAllSelected ? 'bg-accent-blue border-accent-blue' : 'border-zinc-500'}">
              {#if isAllSelected}
                <svg class="w-2.5 h-2.5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/></svg>
              {/if}
            </div>
            <span>全选</span>
          </button>

          <div class="flex-1 relative">
            <svg class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
            <input
              type="text"
              bind:value={filterText}
              placeholder="搜索请求链接、标题或参数..."
              class="w-full bg-zinc-950 border border-zinc-800 rounded-md pl-9 pr-3 py-1.5 text-xs text-zinc-200 outline-none focus:border-accent-blue transition-colors"
            />
          </div>

          <button
            class="px-4 py-1.5 bg-accent-blue hover:bg-blue-600 text-white text-xs font-medium rounded-md disabled:opacity-30 disabled:cursor-not-allowed transition-colors shrink-0"
            disabled={selectedUrls.size === 0}
            onclick={handleBatchDownload}
          >
            批量提取 ({selectedUrls.size})
          </button>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-4 space-y-2">
        {#if filteredResources.length === 0}
          <div class="h-full flex flex-col items-center justify-center text-zinc-600 space-y-2">
            <svg class="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/></svg>
            <span class="text-xs">无匹配记录</span>
          </div>
        {:else}
          {#each filteredResources as res (res.url)}
            <div
              class="flex flex-col p-3 bg-zinc-800/30 border {res.is_highlighted ? 'border-amber-500/50 bg-amber-500/10' : (selectedUrls.has(res.url) ? 'border-accent-blue/50 bg-accent-blue/5' : 'border-zinc-800')} rounded-lg hover:bg-zinc-800/60 transition-colors cursor-pointer"
              onclick={() => toggleSelection(res.url)}
            >
              <div class="flex items-start">
                <div class="mt-1 mr-3 shrink-0">
                  <div class="w-4 h-4 rounded border flex items-center justify-center transition-colors {selectedUrls.has(res.url) ? (res.is_highlighted ? 'bg-amber-500 border-amber-500' : 'bg-accent-blue border-accent-blue') : 'border-zinc-600'}">
                    {#if selectedUrls.has(res.url)}
                      <svg class="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/></svg>
                    {/if}
                  </div>
                </div>

                <div class="w-14 h-10 shrink-0 bg-zinc-950 rounded border border-zinc-800/50 mr-3 overflow-hidden flex items-center justify-center">
                  {#if res.category === 'image'}
                    <img src={res.url} alt="preview" class="w-full h-full object-cover" />
                  {:else}
                    <svg class="w-5 h-5 text-zinc-700" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      {#if res.category === 'media'}
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/>
                      {:else if res.category === 'xhr/fetch'}
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"/>
                      {:else}
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                      {/if}
                    </svg>
                  {/if}
                </div>

                <div class="flex-1 min-w-0 pr-4">
                  <h4 class="text-xs font-medium {res.is_highlighted ? 'text-amber-400' : 'text-zinc-200'} truncate">{taskStore.parseTemplate(res)}</h4>
                  <p class="text-[9px] text-zinc-500 mt-1 truncate font-mono">来源: {res.page_title || '未知网页'}</p>
                  <p class="text-[9px] text-zinc-400 mt-0.5 truncate font-mono opacity-60">{res.url}</p>

                  <div class="flex gap-2 mt-2 items-center flex-wrap">
                    {#if res.method}
                      <span class="px-1.5 py-0.5 bg-zinc-700/80 text-zinc-300 border border-zinc-600 rounded text-[9px] font-bold">{res.method}</span>
                    {/if}
                    <span class="px-1.5 py-0.5 bg-zinc-700/50 text-accent-blue border border-zinc-700 rounded text-[9px] uppercase">{res.ext || res.type}</span>
                    <span class="px-1.5 py-0.5 bg-zinc-800 text-zinc-400 border border-zinc-700/50 rounded text-[9px] font-mono">{formatSize(res.size)}</span>
                    {#if res.category}
                      <span class="px-1.5 py-0.5 bg-zinc-700/50 text-purple-400 border border-purple-900/50 rounded text-[9px]">{res.category}</span>
                    {/if}
                    {#if res.headers?.Cookie}
                      <span class="px-1.5 py-0.5 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-[9px]">含鉴权</span>
                    {/if}
                  </div>
                </div>

                <button
                  class="shrink-0 px-3 py-1.5 {res.is_highlighted ? 'bg-amber-500 hover:bg-amber-400 text-black' : 'bg-zinc-200 hover:bg-white text-zinc-900'} text-xs font-bold rounded transition-colors"
                  onclick={(e) => { e.stopPropagation(); handleDownload(res); }}
                >
                  提取
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>