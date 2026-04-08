<script lang="ts">
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import { IPC } from '$lib/api/ipc';
  import ProgressBar from '$lib/components/ui/ProgressBar.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import type { MediaInfo } from '$lib/types';

  let activeTab = $state<'all' | 'active' | 'pausedOrError'>('all');
  let showNewTaskModal = $state(false);
  
  // URL 输入流状态
  let inputUrl = $state('');
  let isParsing = $state(false);
  let parseError = $state('');

  // 浏览器 Cookie 选项
  const browsers = [
    { id: 'none', label: '不使用 Cookie' },
    { id: 'chrome', label: 'Google Chrome' },
    { id: 'edge', label: 'Microsoft Edge' },
    { id: 'firefox', label: 'Mozilla Firefox' },
    { id: 'safari', label: 'Apple Safari' },
    { id: 'brave', label: 'Brave Browser' }
  ];

  // 合集选择流状态
  let parsedInfo = $state<MediaInfo | null>(null);
  let showPlaylistModal = $state(false);
  let selectedItems = $state<Set<number>>(new Set());

  // 当前激活的任务列表视图
  let displayTasks = $derived.by(() => {
    switch (activeTab) {
      case 'active': return taskStore.activeTasks;
      case 'pausedOrError': return taskStore.pausedOrErrorTasks;
      default: return taskStore.taskList.filter(t => t.status !== 'completed');
    }
  });

  // 第一步：解析 URL
  async function handleParse() {
    if (!inputUrl) return;
    parseError = '';
    isParsing = true;
    
    try {
      const info = await IPC.parseUrl(inputUrl);
      parsedInfo = info;
      
      // 判断是否为合集
      if (info.playlist_entries && info.playlist_entries.length > 1) {
        // 打开合集勾选界面
        showNewTaskModal = false;
        // 默认全选
        selectedItems = new Set(info.playlist_entries.map((_, i) => i + 1));
        showPlaylistModal = true;
      } else {
        // 单视频，直接提交
        showNewTaskModal = false;
        const tempId = taskStore.createTempTask(inputUrl);
        await taskStore.commitTask(tempId, inputUrl, info);
        inputUrl = '';
      }
    } catch (e: any) {
      parseError = e?.toString() || '解析失败，请检查链接或网络';
    } finally {
      isParsing = false;
    }
  }

  // 第二步：提交合集勾选
  async function handleCommitPlaylist() {
    if (!parsedInfo || selectedItems.size === 0) return;
    
    const itemsArray = Array.from(selectedItems).sort((a, b) => a - b);
    
    // 将数组 [1, 2, 4, 5] 简化为 yt-dlp 支持的连续字符串 (略: 这里直接使用逗号分隔即可，如 "1,2,4,5")
    const playlistItemsStr = itemsArray.join(',');
    
    showPlaylistModal = false;
    const tempId = taskStore.createTempTask(inputUrl);
    await taskStore.commitTask(tempId, inputUrl, parsedInfo, playlistItemsStr);
    
    inputUrl = '';
    parsedInfo = null;
    selectedItems.clear();
  }

  function toggleSelectAll() {
    if (!parsedInfo?.playlist_entries) return;
    if (selectedItems.size === parsedInfo.playlist_entries.length) {
      selectedItems.clear();
    } else {
      selectedItems = new Set(parsedInfo.playlist_entries.map((_, i) => i + 1));
    }
  }

  function toggleItem(index: number) {
    if (selectedItems.has(index)) {
      selectedItems.delete(index);
    } else {
      selectedItems.add(index);
    }
    // 强制触发 Svelte 5 响应式更新 (Set 突变需要重新赋值)
    selectedItems = new Set(selectedItems);
  }

  // 任务快捷操作
  async function handleToggleTask(taskId: string, status: string) {
    try {
      if (status === 'paused' || status === 'error') {
        taskStore.update(taskId, { status: 'pending' });
        await IPC.resumeTask(taskId);
      } else {
        taskStore.update(taskId, { status: 'paused' });
        await IPC.pauseTask(taskId);
      }
    } catch (e) { console.error('操作任务状态失败:', e); }
  }

  async function handleDeleteTask(taskId: string) {
    try {
      taskStore.remove(taskId);
      await IPC.cancelTask(taskId);
    } catch (e) { console.error('删除任务失败:', e); }
  }
</script>

<div class="h-full flex flex-col relative">
  <header class="shrink-0 px-6 py-4 flex items-center justify-between border-b border-zinc-800/50">
    <div class="flex space-x-1 bg-zinc-800/50 p-1 rounded-lg">
      {#each [
        { id: 'all', label: '全部任务' },
        { id: 'active', label: '下载中' },
        { id: 'pausedOrError', label: '已暂停/错误' }
      ] as tab}
        <button
          class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors {activeTab === tab.id ? 'bg-zinc-700 text-zinc-100 shadow-sm' : 'text-zinc-400 hover:text-zinc-200'}"
          onclick={() => activeTab = tab.id as any}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <button
      class="flex items-center space-x-1 px-3 py-1.5 bg-accent-blue text-white text-xs font-medium rounded-lg hover:bg-blue-600 transition-colors shadow-sm"
      onclick={() => { showNewTaskModal = true; parseError = ''; }}
    >
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
      <span>新建下载</span>
    </button>
  </header>

  <div class="flex-1 overflow-y-auto p-4 space-y-3">
    {#if displayTasks.length === 0}
      <div class="h-full flex flex-col items-center justify-center text-zinc-500 space-y-2">
        <svg class="w-12 h-12 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/></svg>
        <p class="text-sm">暂无任务</p>
      </div>
    {:else}
      {#each displayTasks as task (task.id)}
        <div class="group flex items-center p-3 bg-zinc-800/20 hover:bg-zinc-800/50 border border-zinc-800 rounded-xl transition-colors">
          <div class="w-20 h-14 shrink-0 bg-zinc-800 rounded-md overflow-hidden mr-4 relative">
            {#if task.thumbnail}
              <img src={task.thumbnail.replace('http://', 'https://')} alt="cover" class="w-full h-full object-cover" />
            {:else}
              <div class="w-full h-full flex items-center justify-center text-zinc-600">
                <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
              </div>
            {/if}
            {#if task.playlist_items}
              <div class="absolute bottom-1 right-1 bg-black/70 px-1 rounded text-[9px] font-mono border border-zinc-700/50">合集</div>
            {/if}
          </div>

          <div class="flex-1 min-w-0 pr-4">
            <h4 class="text-sm font-medium text-zinc-200 truncate mb-2">{task.title}</h4>
            <ProgressBar
              progress={task.total_bytes > 0 ? task.downloaded_bytes / task.total_bytes : task.downloaded_bytes / 100}
              speedText={task.speed > 0 ? (task.speed / 1024 / 1024).toFixed(2) + " MB/s" : (task.status === 'downloading' ? "测速中..." : "")}
              etaText={task.eta > 0 ? task.eta + "s" : ""}
              sizeText={task.total_bytes > 0 ? (task.total_bytes / 1024 / 1024).toFixed(1) + " MB" : ""}
              status={task.status}
            />
          </div>

          <div class="shrink-0 flex items-center space-x-2 opacity-0 group-hover:opacity-100 transition-opacity">
            {#if task.status !== 'completed'}
              <button
                class="w-8 h-8 flex items-center justify-center rounded-full bg-zinc-700/50 hover:bg-zinc-600 text-zinc-300"
                onclick={() => handleToggleTask(task.id, task.status)}
              >
                {#if task.status === 'paused' || task.status === 'error'}
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
                {:else}
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                {/if}
              </button>
            {/if}
            <button
              class="w-8 h-8 flex items-center justify-center rounded-full bg-zinc-700/50 hover:bg-red-500/80 text-zinc-300 hover:text-white"
              onclick={() => handleDeleteTask(task.id)}
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- 模态框 1：新建任务（带 Cookie 选择） -->
  <Modal show={showNewTaskModal} title="新建下载任务" onclose={() => showNewTaskModal = false}>
    <div class="space-y-4">
      <div class="flex space-x-2">
        <!-- 浏览器 Cookie 下拉框 -->
        <select 
          class="shrink-0 bg-zinc-950 border border-zinc-700 rounded-lg px-3 py-3 text-xs text-zinc-300 outline-none focus:border-accent-blue"
          bind:value={configStore.settings.browser_cookie}
          onchange={() => configStore.update({ browser_cookie: configStore.settings.browser_cookie })}
        >
          {#each browsers as b}
            <option value={b.id}>{b.label}</option>
          {/each}
        </select>
        
        <div class="relative flex-1">
          <input
            type="text"
            bind:value={inputUrl}
            placeholder="粘贴视频或合集链接"
            class="w-full bg-zinc-950 border border-zinc-700 focus:border-accent-blue rounded-lg pl-4 pr-20 py-3 text-sm text-zinc-100 outline-none transition-colors"
            onkeydown={(e) => e.key === 'Enter' && !isParsing && handleParse()}
          />
          <button
            class="absolute right-1.5 top-1.5 bottom-1.5 px-4 bg-accent-blue hover:bg-blue-600 text-white text-xs font-medium rounded-md transition-colors disabled:opacity-50"
            onclick={handleParse}
            disabled={!inputUrl || isParsing}
          >
            {isParsing ? '解析中' : '解析'}
          </button>
        </div>
      </div>
      
      {#if parseError}
        <div class="text-xs text-red-400 bg-red-400/10 p-2 rounded border border-red-400/20 break-words">
          {parseError}
        </div>
      {/if}
      <div class="text-[11px] text-zinc-500">
        💡 提示：使用对应浏览器 Cookie 解析可突破 B站 1080P 或高画质会员限制。
      </div>
    </div>
  </Modal>

  <!-- 模态框 2：合集列表项选择 -->
  <Modal show={showPlaylistModal} title="合集下载选择" onclose={() => showPlaylistModal = false}>
    <div class="space-y-4 flex flex-col h-[50vh]">
      <div class="flex justify-between items-end shrink-0">
        <div>
          <h4 class="text-sm font-medium text-zinc-200 line-clamp-1" title={parsedInfo?.title}>{parsedInfo?.title}</h4>
          <p class="text-xs text-zinc-500 mt-1">共 {parsedInfo?.playlist_entries?.length || 0} 个项目 · 已选 <span class="text-accent-blue">{selectedItems.size}</span> 个</p>
        </div>
        <button 
          class="text-xs text-zinc-400 hover:text-zinc-200 border border-zinc-700 px-3 py-1 rounded"
          onclick={toggleSelectAll}
        >
          全选 / 反选
        </button>
      </div>

      <div class="flex-1 overflow-y-auto border border-zinc-800 rounded-lg bg-zinc-950 p-2 space-y-1">
        {#if parsedInfo?.playlist_entries}
          {#each parsedInfo.playlist_entries as entry, i}
            {@const idx = entry.playlist_index || (i + 1)}
            <button 
              class="w-full flex items-center space-x-3 p-2 rounded hover:bg-zinc-800/50 transition-colors text-left"
              onclick={() => toggleItem(idx)}
            >
              <div class="w-4 h-4 shrink-0 rounded border {selectedItems.has(idx) ? 'bg-accent-blue border-accent-blue' : 'border-zinc-600'} flex items-center justify-center">
                {#if selectedItems.has(idx)}
                  <svg class="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/></svg>
                {/if}
              </div>
              <span class="text-xs text-zinc-500 w-6 shrink-0">{idx}.</span>
              <span class="text-sm text-zinc-300 truncate flex-1">{entry.title}</span>
            </button>
          {/each}
        {/if}
      </div>

      <div class="shrink-0 pt-2 flex justify-end space-x-2">
        <button 
          class="px-4 py-2 text-sm text-zinc-400 hover:text-zinc-200"
          onclick={() => showPlaylistModal = false}
        >
          取消
        </button>
        <button 
          class="px-5 py-2 bg-accent-blue hover:bg-blue-600 text-white text-sm font-medium rounded-lg disabled:opacity-50"
          disabled={selectedItems.size === 0}
          onclick={handleCommitPlaylist}
        >
          添加至下载队列
        </button>
      </div>
    </div>
  </Modal>
</div>