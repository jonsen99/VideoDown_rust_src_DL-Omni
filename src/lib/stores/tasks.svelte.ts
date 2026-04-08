import type { Task, MediaInfo } from '$lib/types';
import { IPC } from '$lib/api/ipc';
import { configStore } from '$lib/stores/config.svelte';

class TaskStore {
  tasks = $state<Record<string, Task>>({});

  taskList = $derived(Object.values(this.tasks));
  
  activeTasks = $derived(
    this.taskList.filter(t => 
      t.status === 'downloading' || t.status === 'pending' || t.status === 'merging'
    )
  );

  completedTasks = $derived(
    this.taskList.filter(t => t.status === 'completed')
  );

  pausedOrErrorTasks = $derived(
    this.taskList.filter(t => t.status === 'paused' || t.status === 'error')
  );

  init(initialTasks: Task[]) {
    const newTasks: Record<string, Task> = {};
    for (const task of initialTasks) {
      newTasks[task.id] = task;
    }
    this.tasks = newTasks;
  }

  add(task: Task) {
    this.tasks[task.id] = task;
  }

  update(id: string, partial: Partial<Task>) {
    if (this.tasks[id]) {
      this.tasks[id] = { ...this.tasks[id], ...partial };
    }
  }

  batchUpdateProgress(updates: Partial<Task>[]) {
    for (const update of updates) {
      if (update.id) {
        this.update(update.id, update);
      }
    }
  }

  remove(id: string) {
    delete this.tasks[id];
  }

  // --- 支持直链与合集交互的新版任务流 ---

  createTempTask(url: string): string {
    const tempId = `temp-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
    this.add({
      id: tempId,
      url: url,
      title: "解析中...",
      thumbnail: undefined,
      status: 'pending',
      format_id: '',
      total_bytes: 0,
      downloaded_bytes: 0,
      speed: 0,
      eta: 0,
      created_at: Date.now(),
      error_msg: undefined
    });
    return tempId;
  }

  async commitTask(tempId: string, url: string, info: MediaInfo, playlistItems?: string) {
    try {
      if (!this.tasks[tempId]) return;
      
      const { split_audio_video, video_quality, audio_quality } = configStore.settings;

      let formatId = 'direct'; // [修改] 默认为直链设置占位符

      // [修改] 如果标识不是 direct_link，说明需要交给 yt-dlp，才组装相关画质参数
      if (info.id !== 'direct_link') {
        const videoFilter = video_quality === 'best'
          ? 'bv*'
          : `bv[height<=${video_quality.replace('p', '')}]`;

        const audioFilter = audio_quality === 'best'
          ? 'ba'
          : `ba[abr<=${audio_quality.replace('k', '')}]`;

        formatId = split_audio_video
          ? `${videoFilter}/${audioFilter}`
          : `${videoFilter}+${audioFilter}/b`;
      }
      
      const title = info.title || "未知标题";
      const thumbnail: string | undefined = info.thumbnail || undefined;
      
      const taskId = await IPC.createTask(url, title, thumbnail, formatId, playlistItems);
      
      if (this.tasks[tempId]) {
        this.remove(tempId);
        this.add({
          id: taskId,
          url: url,
          title: title,
          thumbnail: thumbnail,
          status: 'pending',
          format_id: formatId,
          playlist_items: playlistItems,
          total_bytes: 0,
          downloaded_bytes: 0,
          speed: 0,
          eta: 0,
          created_at: Date.now(),
          error_msg: undefined
        });
      }
    } catch (e: any) {
      console.error('生成任务失败:', e);
      if (this.tasks[tempId]) {
        this.update(tempId, { 
          status: 'error', 
          title: '创建任务失败',
          error_msg: e?.toString() || '未知错误'
        });
      }
    }
  }

  async submitNewTask(url: string) {
    const tempId = this.createTempTask(url);
    try {
      const info = await IPC.parseUrl(url);
      await this.commitTask(tempId, url, info);
    } catch (e: any) {
      this.update(tempId, { status: 'error', title: '解析失败', error_msg: e?.toString() });
    }
  }
}

export const taskStore = new TaskStore();