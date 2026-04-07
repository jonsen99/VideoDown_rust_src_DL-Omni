<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const appWindow = getCurrentWindow();

  function minimize() {
    appWindow.minimize();
  }

  function toggleMaximize() {
    appWindow.toggleMaximize();
  }

  function close() {
    appWindow.close();
  }
</script>

<div 
  data-tauri-drag-region 
  class="h-8 w-full flex items-center justify-between px-3 bg-zinc-900 border-b border-zinc-800/50 select-none"
>
  <div class="flex items-center space-x-2 pointer-events-none">
    <span class="text-[10px] font-medium text-zinc-500 uppercase tracking-wider">DL-Omni</span>
  </div>

  <div class="flex items-center space-x-1" data-tauri-drag-region>
    <button 
      onclick={minimize}
      aria-label="Minimize"
      title="Minimize"
      class="w-7 h-6 flex items-center justify-center rounded text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors"
      style="-webkit-app-region: no-drag;"
    >
      <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4"/></svg>
    </button>
    <button 
      onclick={toggleMaximize}
      aria-label="Maximize"
      title="Maximize"
      class="w-7 h-6 flex items-center justify-center rounded text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors"
      style="-webkit-app-region: no-drag;"
    >
      <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4"/></svg>
    </button>
    <button 
      onclick={close}
      aria-label="Close"
      title="Close"
      class="w-7 h-6 flex items-center justify-center rounded text-zinc-400 hover:bg-red-500 hover:text-white transition-colors"
      style="-webkit-app-region: no-drag;"
    >
      <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
    </button>
  </div>
</div>