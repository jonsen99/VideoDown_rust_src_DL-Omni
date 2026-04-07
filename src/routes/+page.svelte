<script lang="ts">
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { IPC } from '$lib/api/ipc';
  import ProgressBar from '$lib/components/ui/ProgressBar.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';

  let activeTab = $state<'all' | 'active' | 'pausedOrError'>('all');
  let showNewTaskModal = $state(false);
  let inputUrl = $state('');

  let displayTasks = $derived.by(() => {
    switch (activeTab) {
      case 'active': return taskStore.activeTasks;
      case 'pausedOrError': return taskStore.pausedOrErrorTasks;
      default: return taskStore.taskList.filter(t => t.status !== 'completed');
    }
  });

  function handleParse() {
    if (!inputUrl) return;
    const currentUrl = inputUrl;
    showNewTaskModal = false;
    inputUrl = '';
    taskStore.submitNewTask(currentUrl);
  }

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

    <div class="flex space-x-2">
      <button
        class="flex items-center space-x-1 px-3 py-1.5 bg-accent-blue text-white text-xs font-medium rounded-lg hover:bg-blue-600 transition-colors shadow-sm"
        onclick={() => showNewTaskModal = true}
      >
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        <span>新建下载</span>
      </button>
    </div>
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
          <div class="w-20 h-14 shrink-0 bg-zinc-800 rounded-md overflow-hidden mr-4">
            {#if task.thumbnail}
              <img src={task.thumbnail.replace('http://', 'https://')} alt="cover" class="w-full h-full object-cover" />
            {:else}
              <div class="w-full h-full flex items-center justify-center text-zinc-600">
                <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
              </div>
            {/if}
          </div>

          <div class="flex-1 min-w-0 pr-4">
            <h4 class="text-sm font-medium text-zinc-200 truncate mb-2">{task.title}</h4>
            <ProgressBar
              progress={task.total_bytes > 0
                ? task.downloaded_bytes / task.total_bytes
                : task.downloaded_bytes / 100}
              speedText={task.speed > 0
                ? (task.speed / 1024 / 1024).toFixed(2) + " MB/s"
                : "测速中..."}
              etaText={task.eta > 0 ? task.eta + "s" : "--"}
              sizeText={task.total_bytes > 0
                ? (task.total_bytes / 1024 / 1024).toFixed(1) + " MB"
                : "正在计算片段..."}
              status={task.status}
            />
          </div>

          <div class="shrink-0 flex items-center space-x-2 opacity-0 group-hover:opacity-100 transition-opacity">
            {#if task.status !== 'completed'}
              <button
                class="w-8 h-8 flex items-center justify-center rounded-full bg-zinc-700/50 hover:bg-zinc-600 text-zinc-300"
                aria-label="切换任务状态"
                title="切换任务状态"
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
              aria-label="删除任务"
              title="删除任务"
              onclick={() => handleDeleteTask(task.id)}
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <Modal show={showNewTaskModal} title="新建下载任务" onclose={() => showNewTaskModal = false}>
    <div class="space-y-4">
      <div class="relative">
        <input
          type="text"
          bind:value={inputUrl}
          placeholder="在此粘贴流媒体链接 (支持 B站, YouTube, 直链等)"
          class="w-full bg-zinc-950 border border-zinc-700 focus:border-accent-blue rounded-lg px-4 py-3 text-sm text-zinc-100 outline-none transition-colors pr-24"
        />
        <button
          class="absolute right-1.5 top-1.5 bottom-1.5 px-4 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium rounded-md transition-colors disabled:opacity-50"
          onclick={handleParse}
          disabled={!inputUrl}
        >
          解析
        </button>
      </div>
    </div>
  </Modal>
</div>