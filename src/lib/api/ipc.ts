import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { MediaInfo, Task } from '$lib/types';
import { taskStore } from '$lib/stores/tasks.svelte';

export const IPC = {
  async parseUrl(url: string): Promise<MediaInfo> {
    return await invoke<MediaInfo>('parse_url', { url });
  },

  async createTask(url: string, title: string, thumbnail: string | undefined, formatId: string): Promise<string> {
    return await invoke<string>('create_task', { url, title, thumbnail, formatId });
  },

  async pauseTask(taskId: string): Promise<void> {
    await invoke('pause_task', { taskId });
  },

  async resumeTask(taskId: string): Promise<void> {
    await invoke('resume_task', { taskId });
  },

  async getAllTasks(): Promise<Task[]> {
    return await invoke<Task[]>('get_all_tasks');
  },

  async cancelTask(taskId: string): Promise<void> {
    await invoke('cancel_task', { taskId });
  },

  async clearHistory(): Promise<void> {
    await invoke('clear_history');
  },

  async openFolder(): Promise<void> {
    await invoke('open_folder');
  },

  // --- 更新：嗅探业务指令层 ---

  async startSniffing(url: string): Promise<void> {
    await invoke('start_sniffing', { url });
  },

  async stopSniffing(): Promise<void> {
    await invoke('stop_sniffing');
  },

  // --- 事件监听层 ---

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

  async listenSniffedResources(callback: (resource: any) => void): Promise<UnlistenFn> {
    return await listen<any>('sniffed_resource', (event) => {
      callback(event.payload);
    });
  }
};