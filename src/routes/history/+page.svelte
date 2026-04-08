<script lang="ts">
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { IPC } from '$lib/api/ipc';
  
  let completedTasks = $derived(taskStore.completedTasks);

  async function handleClearHistory() {
    try {
      // Optimistically clear store
      const tasksToClear = completedTasks.map(t => t.id);
      tasksToClear.forEach(id => taskStore.remove(id));
      await IPC.clearHistory();
    } catch (e) {
      console.error('清空历史记录失败:', e);
    }
  }

  async function handleOpenFolder() {
    try {
      await IPC.openFolder();
    } catch (e) {
      console.error('打开文件夹失败:', e);
    }
  }
</script>

<div class="h-full flex flex-col p-6 space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-medium text-zinc-100 tracking-wide">完成历史</h2>
    <button 
      class="text-xs font-medium text-accent-red hover:text-red-400 transition-colors"
      onclick={handleClearHistory}
    >
      清空记录
    </button>
  </div>

  <div class="flex-1 overflow-y-auto space-y-2">
    {#if completedTasks.length === 0}
      <div class="h-full flex items-center justify-center text-sm text-zinc-500">
        尚无下载完成的任务
      </div>
    {:else}
      {#each completedTasks as task}
        <div class="flex items-center justify-between p-4 bg-zinc-800/30 border border-zinc-800 rounded-lg hover:bg-zinc-800/50 transition-colors">
          <div class="flex items-center space-x-4 overflow-hidden">
            <div class="w-14 h-10 rounded bg-zinc-800 flex items-center justify-center shrink-0 overflow-hidden">
              {#if task.thumbnail}
                <img src={task.thumbnail.replace('http://', 'https://')} alt="cover" class="w-full h-full object-cover" />
              {:else}
                <svg class="w-5 h-5 text-zinc-600" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
              {/if}
            </div>
            <div class="min-w-0">
              <h4 class="text-sm font-medium text-zinc-200 truncate">{task.title}</h4>
              <p class="text-xs text-zinc-500 mt-1">{new Date(task.created_at).toLocaleString()}</p>
            </div>
          </div>
          <div class="flex items-center space-x-3 shrink-0 ml-4">
            <button 
              class="px-3 py-1.5 text-xs font-medium bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded transition-colors"
              onclick={handleOpenFolder}
            >
              打开文件夹
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>