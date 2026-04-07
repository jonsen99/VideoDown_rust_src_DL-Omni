use rusqlite::{params, Connection, Result as SqlResult};
use tauri::AppHandle;
use tauri::Manager;
use std::path::PathBuf;
use crate::models::{Task, TaskStatus};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn init(app: &AppHandle) -> Result<Self, rusqlite::Error> {
        let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("./"));
        std::fs::create_dir_all(&app_dir).ok();
        
        let db_path = app_dir.join("tasks.db");
        let conn = Connection::open(db_path)?;

        // 开启 WAL 模式，防止高频读写导致数据库锁死
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        // 初始化表结构
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                thumbnail TEXT,
                status TEXT NOT NULL,
                format_id TEXT NOT NULL,
                total_bytes INTEGER DEFAULT 0,
                downloaded_bytes INTEGER DEFAULT 0,
                speed REAL DEFAULT 0.0,
                eta INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                error_msg TEXT
            )",
            [],
        )?;

        let mut db = Self { conn };
        db.recover_orphan_tasks()?; // 处理异常退出导致的僵尸任务
        Ok(db)
    }

    /// 插入新任务
    pub fn insert_task(&self, task: &Task) -> SqlResult<()> {
        let status_str = serde_json::to_string(&task.status).unwrap_or_default().replace("\"", "");
        self.conn.execute(
            "INSERT INTO tasks (id, url, title, thumbnail, status, format_id, total_bytes, downloaded_bytes, speed, eta, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.id, task.url, task.title, task.thumbnail, status_str, task.format_id,
                task.total_bytes, task.downloaded_bytes, task.speed, task.eta, task.created_at
            ],
        )?;
        Ok(())
    }

    /// 更新任务状态
    pub fn update_status(&self, id: &str, status: TaskStatus) -> SqlResult<()> {
        let status_str = serde_json::to_string(&status).unwrap_or_default().replace("\"", "");
        self.conn.execute("UPDATE tasks SET status = ?1 WHERE id = ?2", params![status_str, id])?;
        Ok(())
    }

    /// 更新任务最终状态和文件大小
    pub fn update_task_finish(&self, id: &str, status: TaskStatus, total_bytes: u64) -> SqlResult<()> {
        let status_str = serde_json::to_string(&status).unwrap_or_default().replace("\"", "");
        self.conn.execute("UPDATE tasks SET status = ?1, total_bytes = ?2, downloaded_bytes = ?2 WHERE id = ?3", params![status_str, total_bytes, id])?;
        Ok(())
    }

    /// 获取所有任务
    pub fn get_all_tasks(&self) -> SqlResult<Vec<Task>> {
        let mut stmt = self.conn.prepare("SELECT * FROM tasks ORDER BY created_at DESC")?;
        let task_iter = stmt.query_map([], |row| {
            let status_str: String = row.get(4)?;
            let status: TaskStatus = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or(TaskStatus::Error);
            
            Ok(Task {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                thumbnail: row.get(3)?,
                status,
                format_id: row.get(5)?,
                total_bytes: row.get(6)?,
                downloaded_bytes: row.get(7)?,
                speed: row.get(8)?,
                eta: row.get(9)?,
                created_at: row.get(10)?,
                error_msg: row.get(11)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in task_iter {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    /// 获取单条任务详情
    pub fn get_task(&self, id: &str) -> SqlResult<Option<Task>> {
        let mut stmt = self.conn.prepare("SELECT * FROM tasks WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let status_str: String = row.get(4)?;
            let status: TaskStatus = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or(TaskStatus::Error);
            
            Ok(Some(Task {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                thumbnail: row.get(3)?,
                status,
                format_id: row.get(5)?,
                total_bytes: row.get(6)?,
                downloaded_bytes: row.get(7)?,
                speed: row.get(8)?,
                eta: row.get(9)?,
                created_at: row.get(10)?,
                error_msg: row.get(11)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 删除任务
    pub fn delete_task(&self, id: &str) -> SqlResult<()> {
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 清空所有已完成的历史任务
    pub fn clear_history(&self) -> SqlResult<()> {
        self.conn.execute("DELETE FROM tasks WHERE status = '\"completed\"' OR status = 'completed'", [])?;
        Ok(())
    }

    /// 恢复意外退出导致仍处于 "downloading" 的孤儿任务为 "paused"
    fn recover_orphan_tasks(&mut self) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE tasks SET status = 'paused', speed = 0.0 WHERE status = 'downloading' OR status = 'merging'",
            [],
        )?;
        Ok(())
    }
}