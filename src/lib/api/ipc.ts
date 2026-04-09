import { invoke } from '@tauri-apps/api/core';
import type { MediaInfo, Task, SniffedResource } from '$lib/types';
import { taskStore } from '$lib/stores/tasks.svelte';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export const IPC = {
  async parseUrl(url: string): Promise<MediaInfo> {
    return await invoke<MediaInfo>('parse_url', { url });
  },

  async createTask(
    url: string, 
    title: string, 
    thumbnail: string | undefined, 
    formatId: string,
    playlistItems?: string,
    httpHeaders?: string 
  ): Promise<string> {
    // 【规范对齐】显式使用 snake_case 键名，严格匹配后端 #[command(rename_all = "snake_case")]
    return await invoke<string>('create_task', { 
      url, 
      title, 
      thumbnail, 
      format_id: formatId, 
      playlist_items: playlistItems,
      http_headers: httpHeaders 
    });
  },

  async pauseTask(taskId: string): Promise<void> {
    // 【规范对齐】严格匹配后端参数 task_id
    await invoke('pause_task', { task_id: taskId });
  },

  async resumeTask(taskId: string): Promise<void> {
    // 【规范对齐】严格匹配后端参数 task_id
    await invoke('resume_task', { task_id: taskId });
  },

  async getAllTasks(): Promise<Task[]> {
    return await invoke<Task[]>('get_all_tasks');
  },

  async cancelTask(taskId: string): Promise<void> {
    // 【规范对齐】严格匹配后端参数 task_id
    await invoke('cancel_task', { task_id: taskId });
  },

  async clearHistory(): Promise<void> {
    await invoke('clear_history');
  },

  async openFolder(): Promise<void> {
    await invoke('open_folder');
  },

  async startSniffing(url: string): Promise<void> {
    await invoke('start_sniffing', { url });
  },

  async stopSniffing(): Promise<void> {
    await invoke('stop_sniffing');
  },

  async listenProgressUpdates(): Promise<UnlistenFn> {
    return await listen<Partial<Task>[]>('batch_progress_update', (event) => {
      taskStore.batchUpdateProgress(event.payload);
    });
  },

  async listenTaskError(): Promise<UnlistenFn> {
    return await listen<{ id: string, error: string }>('task_error', (event) => {
      const { id, error } = event.payload;
      taskStore.update(id, { status: 'error', error_msg: error });
    });
  },

  async listenSniffedResources(callback: (resource: SniffedResource) => void): Promise<UnlistenFn> {
    return await listen<SniffedResource>('sniffed_resource', (event) => {
      callback(event.payload);
    });
  }
};