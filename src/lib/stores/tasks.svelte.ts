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

  // --- 【重构】支持合集交互的新版任务流 ---

  /**
   * 1. 占位：在 UI 列表创建一条“解析中”的临时任务
   */
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

  /**
   * 2. 确认创建：由 UI 调用 (用户可能已在合集弹窗中勾选了子集)
   */
  async commitTask(tempId: string, url: string, info: MediaInfo, playlistItems?: string) {
    try {
      if (!this.tasks[tempId]) return; // 若临时任务被用户手快删了，则中断
      
      const { split_audio_video, video_quality, audio_quality } = configStore.settings;

      const videoFilter = video_quality === 'best'
        ? 'bv*'
        : `bv[height<=${video_quality.replace('p', '')}]`;

      const audioFilter = audio_quality === 'best'
        ? 'ba'
        : `ba[abr<=${audio_quality.replace('k', '')}]`;

      const formatId = split_audio_video
        ? `${videoFilter}/${audioFilter}`
        : `${videoFilter}+${audioFilter}/b`;
      
      const title = info.title || "未知标题";
      const thumbnail: string | undefined = info.thumbnail || undefined;
      
      // IPC 创建正式任务，附带合集参数
      const taskId = await IPC.createTask(url, title, thumbnail, formatId, playlistItems);
      
      // 替换临时任务
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

  /**
   * 提供给外部系统的简单快捷入口 (比如嗅探器直接发起单视频下载)
   */
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