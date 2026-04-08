这个项目（DL-Omni）的架构设计非常现代化，结合了 Tauri 2、Svelte 5 的响应式特性（Runes）以及 Rust 的高性能多线程处理，整体的模块划分也很清晰。

经过全面梳理，我发现了代码中存在一些**未完全实现的功能**、**严重的鲁棒性缺陷（主要集中在 Rust 异步任务管理和文件操作上）**，以及部分**可读性/边界处理不足**的地方。

按照您的要求，本次**只做分析和列出需要修改的文件，不输出任何具体代码**。

---

### 一、 尚未实现的功能 (Missing Features)

**1. 代理配置（Proxy）全链路缺失**
* **现象：** README 和 `models.rs` 的 `Config` 结构体中都定义了 `proxy_url`，但在后端引擎和前端 UI 中均未落地。
* **影响：** * 前端 `settings/+page.svelte` 中缺少代理的输入框。
    * `downloader.rs` 中的 `reqwest::Client` 没有挂载代理配置。
    * `ytdlp.rs` 中的 `Command` 没有传递 `--proxy` 参数。
    * `updater.rs` 下载更新核心时，如果用户处于特殊网络环境，更新会直接失败。

**2. 核心文件更新时的进程查杀（TODO 未完成）**
* **现象：** `updater.rs` 的 `check_and_update` 函数中留有一个 `TODO: 核心防错 - 在此处遍历系统进程，kill 掉所有名为 yt-dlp 的孤儿进程...`。
* **影响：** 在 Windows 系统下，如果用户后台还有未完成的下载任务（`yt-dlp.exe` 正在运行），此时触发引擎更新，`fs::rename` 会因为文件被占用而抛出“权限拒绝 (Access is denied)”错误，导致更新逻辑崩溃。

---

### 二、 严重的鲁棒性缺陷 (Severe Robustness Issues)

这部分问题可能导致应用崩溃、内存泄漏或产生幽灵进程，需要优先修复。

**1. `yt-dlp` 幽灵进程（内存/带宽泄漏）**
* **现象：** 在 `commands.rs` 中，`pause_task` 和 `cancel_task` 调用了 `handle.abort()` 来终止 Tokio 的异步任务。
* **根本原因：** `handle.abort()` 只能取消 Rust 层的 Future 轮询。但在 `ytdlp.rs` 中，通过 `tokio::process::Command` 衍生出的操作系统子进程（`yt-dlp`）如果不显式配置“随 Future 销毁而终止”，它将在后台继续默默下载，消耗用户的带宽和 CPU，而前端却显示已暂停或已删除。
* **修复方向：** 需要在 `Command::new` 时配置特定的属性，确保丢弃 Future 时操作系统级查杀该子进程。

**2. 异步写文件时的 `unwrap()` 恐慌 (Panic)**
* **现象：** 在 `downloader.rs` 的 `writer_handle` 中，打开文件使用了 `.await.unwrap()`。
* **根本原因：** 如果在下载途中，用户手动在资源管理器中删除了该文件，或者杀毒软件暂时锁定了该文件，这里的 `unwrap()` 会直接导致所在的 Tokio Worker 线程 Panic（恐慌），不仅当前下载崩溃，甚至可能带走整个 Tauri 后端进程。
* **修复方向：** 应该用 `match` 或 `if let` 妥善处理 `Result`，如果文件打开失败，通过 channel 安全退出线程并抛出错误，而不是让线程崩溃。

**3. 文件名长度溢出限制缺失**
* **现象：** `utils::sanitize_filename` 仅过滤了 `[\\/:*?"<>|]` 等非法字符。
* **根本原因：** YouTube 或 Bilibili 的部分视频标题极其冗长（甚至包含大量 emoji）。Windows API 默认存在 260 个字符的路径长度限制（MAX_PATH）。如果路径加文件名超长，`fs::File::create` 会直接返回系统错误，导致任务生成失败。
* **修复方向：** 在过滤非法字符后，需要增加对字符串长度的安全截断（例如最多保留 150 个字符）。

---

### 三、 边界逻辑与可读性优化 (Edge Cases & Readability)

**1. 前端进度条溢出 (UI 异常)**
* **现象：** `ProgressBar.svelte` 和 `+page.svelte` 中的进度计算：当 `task.total_bytes` 为 0（例如遇到无法解析总大小的直播流或分片流）时，使用了 fallback 逻辑 `task.downloaded_bytes / 100`。
* **根本原因：** 如果下载量超过 100 字节，这个比值就会大于 1.0。传递给 UI 的 CSS `transform: scaleX(>1)` 会导致进度条冲出容器边界，破坏布局。
* **修复方向：** 对于未知大小的流，应当传入一个特定的状态（如无限流动条），或者通过 `Math.min(value, 1.0)` 严格限制最大值为 1。

**2. SQLite 高频并发写入锁竞争**
* **现象：** `database.rs` 中 `update_status` 被频繁调用。
* **潜在隐患：** 虽然开启了 WAL 模式，但如果有多个任务同时结束或出错，多个线程同时请求 `state.db.lock().await` 写入状态，仍然存在微小的锁竞争开销。目前由于任务量不大，不至于崩溃，但从架构上看，状态更新最好也走一个专门的异步 Channel 排队处理，或者批量持久化，提高并发可读性。

**3. 嗅探器正则过于脆弱**
* **现象：** `sniffer.rs` 中为了匹配特定平台注入了硬编码的正则（如 `"url_list"`）。
* **潜在隐患：** 随着目标网站前端经常混淆或更新，这段写死在 Rust 字符串里的 JS 极易失效。
* **修复方向：** 建议将注入脚本抽离为单独的 `.js` 静态资源文件读取，或者增加一套动态更新特征库的机制，提高代码的可维护性。

---

### 四、 需要修改的文件清单

如果您决定进行修复，请重点关注并修改以下文件：

**后端 (Rust):**
1.  `src-tauri/src/engine/ytdlp.rs` (修复幽灵进程，添加代理支持)
2.  `src-tauri/src/engine/downloader.rs` (移除 `unwrap()`，增加代理支持)
3.  `src-tauri/src/engine/updater.rs` (实现旧进程清理，添加代理支持)
4.  `src-tauri/src/utils.rs` (增强 `sanitize_filename` 的长度限制)
5.  `src-tauri/src/engine/sniffer.rs` (优化注入脚本的健壮性)

**前端 (Svelte):**
1.  `src/routes/settings/+page.svelte` (补全代理配置 UI)
2.  `src/routes/+page.svelte` (修复 `ProgressBar` 传值的溢出问题)

---

