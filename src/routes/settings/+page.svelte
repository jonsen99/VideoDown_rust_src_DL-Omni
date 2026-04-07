<script lang="ts">
  import { configStore } from '$lib/stores/config.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  let config = $derived(configStore.settings);
  let isCheckingUpdate = $state(false);

  async function selectDirectory() {
    // 调用 Tauri 原生文件选择器
    const selected = await open({ directory: true });
    if (selected) {
      configStore.update({ default_download_path: selected as string });
    }
  }

  async function checkUpdate() {
    isCheckingUpdate = true;
    try {
      const v = await invoke<string>('check_engine_update');
      configStore.update({ yt_dlp_version: v });
      alert(`已更新到引擎版本: ${v}`);
    } catch(e) {
      alert(`更新失败: ${e}`);
    } finally {
      isCheckingUpdate = false;
    }
  }
</script>

<div class="h-full overflow-y-auto p-8 max-w-3xl mx-auto space-y-10">
  <h2 class="text-2xl font-medium text-zinc-100 tracking-wide">全局设置</h2>

  <section class="space-y-4">
    <h3 class="text-xs font-bold text-zinc-500 uppercase tracking-wider">基础存储</h3>
    
    <div class="p-5 bg-zinc-800/30 border border-zinc-800 rounded-xl space-y-4">
      <div class="flex justify-between items-center">
        <div>
          <div class="text-sm font-medium text-zinc-200">默认下载路径</div>
          <div class="text-xs text-zinc-500 mt-1">{config.default_download_path || '未设置 (默认保存至系统 Downloads)'}</div>
        </div>
        <button 
          class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-sm font-medium text-zinc-200 rounded-lg transition-colors border border-zinc-700/50"
          onclick={selectDirectory}
        >
          更改目录
        </button>
      </div>
    </div>
  </section>

  <section class="space-y-4">
    <h3 class="text-xs font-bold text-zinc-500 uppercase tracking-wider">引擎与网络控制</h3>
    
    <div class="p-5 bg-zinc-800/30 border border-zinc-800 rounded-xl space-y-6">
      
      <div class="flex justify-between items-center">
        <div>
          <div class="text-sm font-medium text-zinc-200">下载格式偏好</div>
          <div class="text-xs text-zinc-500 mt-1">设置默认是否同时下载视频与音频</div>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input 
            type="checkbox" 
            class="sr-only peer" 
            checked={config.include_audio}
            onchange={(e) => configStore.update({ include_audio: e.currentTarget.checked })}
          >
          <div class="w-11 h-6 bg-zinc-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-accent-blue"></div>
        </label>
      </div>
      
      <hr class="border-zinc-800">

      <div>
        <div class="flex justify-between text-sm mb-2">
          <span class="font-medium text-zinc-200">最大同时下载任务数</span>
          <span class="text-accent-blue font-mono">{config.max_concurrent_tasks}</span>
        </div>
        <input 
          type="range" min="1" max="10" 
          value={config.max_concurrent_tasks}
          oninput={(e) => configStore.update({ max_concurrent_tasks: parseInt(e.currentTarget.value) })}
          class="w-full accent-accent-blue"
        />
      </div>

      <div>
        <div class="flex justify-between text-sm mb-2">
          <span class="font-medium text-zinc-200">单任务最大线程数 (直链/分片)</span>
          <span class="text-accent-blue font-mono">{config.max_threads_per_task}</span>
        </div>
        <input 
          type="range" min="1" max="32" 
          value={config.max_threads_per_task}
          oninput={(e) => configStore.update({ max_threads_per_task: parseInt(e.currentTarget.value) })}
          class="w-full accent-accent-blue"
        />
      </div>
      
      <hr class="border-zinc-800">

      <div class="flex justify-between items-center">
        <div>
          <div class="text-sm font-medium text-zinc-200">yt-dlp 核心引擎</div>
          <div class="text-xs text-zinc-500 mt-1">当前版本: {config.yt_dlp_version || '未知'}</div>
        </div>
        <button 
          class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-sm font-medium text-zinc-200 rounded-lg transition-colors border border-zinc-700/50"
          onclick={checkUpdate}
          disabled={isCheckingUpdate}
        >
          {isCheckingUpdate ? '正在检查并更新...' : '检查核心更新'}
        </button>
      </div>
    </div>
  </section>
</div>