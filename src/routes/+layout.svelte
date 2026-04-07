<script lang="ts">
  import { onMount } from 'svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { IPC } from '$lib/api/ipc';
  import '../app.css';
  // 未来此处将导入 TitleBar 和 Sidebar 组件
  import TitleBar from '$lib/components/layout/TitleBar.svelte';
  import Sidebar from '$lib/components/layout/Sidebar.svelte';

  // Svelte 5 渲染 children 的标准语法
  let { children } = $props();

  onMount(async () => {
    configStore.init();
    try {
      const tasks = await IPC.getAllTasks();
      console.log('Successfully loaded tasks from DB:', tasks);
      taskStore.init(tasks);

      // 启动全局事件监听器
      await IPC.listenProgressUpdates();
      await IPC.listenTaskError();
    } catch (e) {
      console.error("加载任务列表失败:", e);
    }
  });
</script>

<div class="flex h-screen w-screen flex-col bg-zinc-900 text-zinc-100 overflow-hidden">
  
  <div class="h-8 w-full shrink-0">
    <TitleBar />
  </div>

  <div class="flex flex-1 overflow-hidden">
    
    <Sidebar />

    <main class="flex-1 overflow-y-auto relative">
      {@render children()}
    </main>

  </div>
</div>