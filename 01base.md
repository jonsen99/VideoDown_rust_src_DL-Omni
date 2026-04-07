
### 功能优化建议

我将建议分为四个层面：**核心下载体验增强**、**UI/UX 交互升华**、**高级与自动化功能**、**生态与健壮性**。

#### 1. 核心下载体验增强 (Core Downloading Experience)

##### 建议 1：智能化的播放列表与批量处理 (Playlist & Batch Handling)

*   **现状分析**：当前设计主要围绕单个视频链接的“精准下载”场景。但用户一个常见的高频需求是下载整个 B 站收藏夹、YouTube 播放列表或一个UP主的所有视频。
*   **优化建议**：
    1.  **自动识别播放列表**：当用户粘贴的 URL 被后端 `yt-dlp -J` 解析后，如果检测到返回的 JSON 中含有 `_type: 'playlist'` 字段，前端弹出的 Modal 窗口应变为“播放列表解析”模式。
    2.  **选择性下载界面**：Modal 中不再是清晰度选择，而是展示一个带有多选框（Checkbox）的视频列表（包含标题和缩略图）。顶部应有“全选”、“反选”、“仅选择前 N 个”等快捷操作。
    3.  **批量任务创建**：用户确认后，前端将选中的视频 `entries` 列表一次性发给后端。后端接收后，在数据库中循环创建多个独立的 `Task` 实例，并将它们归属到同一个 `batch_id` 下。这便于未来对整批任务进行统一管理（如：整批暂停）。

*   **技术实现补充**：
    *   Rust 后端：`create_task` 指令需要重载或新增一个 `create_batch_tasks` 指令来接收视频列表。
    *   yt-dlp 命令：可以使用 `--flat-playlist` 标志来快速获取播放列表信息，而无需解析每个视频的详细元数据，提高响应速度。

##### 建议 2：更精细的格式选择与后期处理 (Granular Format Selection & Post-processing)

*   **现状分析**：目前 `format_id` 的选择被简化了（如 `"bv*+ba/b"`），这很适合新手。但高级用户可能需要下载特定编码（如 AV1）、特定音轨或纯音频。
*   **优化建议**：
    1.  **格式详情面板**：在解析成功后的 Modal 中，除了默认推荐的最佳格式外，提供一个“高级”或“显示所有格式”的按钮。点击后，以表格形式清晰展示 `yt-dlp` 返回的 `formats` 数组中的所有可用格式，列出：`分辨率`、`FPS`、`视频编码(vcodec)`、`音频编码(acodec)`、`文件大小`、`备注(format_note)`等。
    2.  **后期处理选项**：在 Modal 中增加“下载选项”区域，提供如：
        *   `[✓] 仅提取音频` -> 并提供音频格式选择（mp3, aac, flac...）。
        *   `[✓] 下载字幕` -> 并提供字幕语言和格式选择（srt, ass...）。
        *   `[✓] 嵌入缩略图与元数据`。

*   **技术实现补充**：
    *   Rust 后端：`create_task` 指令需要增加一个 `post_processing_options` 参数。`ytdlp.rs` 中的下载命令将根据这些选项动态拼接参数，如 `--extract-audio --audio-format mp3`、`--write-subs --sub-lang en` 等。

#### 2. UI/UX 交互升华 (UI/UX Refinement)

##### 建议 3：系统级集成与无缝反馈 (System Integration & Seamless Feedback)

*   **现状分析**：应用窗口关闭或最小化后，用户对下载进度的感知会中断。
*   **优化建议**：
    1.  **系统托盘图标**：创建一个系统托盘（System Tray）图标。
        *   **动态图标**：托盘图标可以根据状态变化，例如空闲时为静态 Logo，下载中时有一个动态旋转或进度动画。
        *   **右键菜单**：提供快速操作，如“显示/隐藏窗口”、“暂停所有任务”、“恢复所有任务”、“退出应用”。
        *   **悬浮提示**：鼠标悬停时显示下载摘要，如 “DL-Omni: 3 个任务正在下载 (15.8 MB/s)”。
    2.  **任务栏/Dock 进度**：在 Windows 任务栏和 macOS Dock 的应用图标上显示全局下载进度条。这是一个非常能提升高级感和实用性的细节。
    3.  **原生通知增强**：使用 `tauri-plugin-notification`，在任务**开始**、**完成**、**失败**时发送系统原生通知。完成的通知可以带有一个“打开所在文件夹”的按钮。

*   **技术实现补充**：
    *   Rust 端：在 `main.rs` 中使用 Tauri 的 `SystemTray` API 构建托盘。通过 AppHandle 在下载引擎中向托盘发送事件来更新其状态。任务栏进度可以通过 `window.set_progress_bar` API 实现。

#### 3. 高级与自动化功能 (Advanced & Automation Features)

##### 建议 4：浏览器扩展联动 (Browser Extension Synergy)

*   **现状分析**：“嗅探模式”非常强大，但仍需用户手动复制 URL 到 DL-Omni。最无缝的体验是在浏览器中“一键推送”。
*   **优化建议**：
    1.  **开发一个极简浏览器扩展** (Chrome/Firefox)。
    2.  **右键菜单集成**：在网页的视频、链接或页面空白处右键，出现“使用 DL-Omni 下载”的选项。
    3.  **一键推送**：点击后，扩展通过 [Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) 将 URL 直接发送给本地运行的 DL-Omni 应用。
    4.  **自动唤起**：DL-Omni 后端接收到消息后，自动弹出“新建下载”窗口并填充好 URL，用户只需确认即可。

*   **技术实现补充**：
    *   Rust 后端：需要配置为 Native Messaging Host，监听来自标准输入（stdin）的浏览器扩展消息。
    *   这是一个相对独立的模块，但能极大提升产品的专业度和使用便捷性。

##### 建议 5：下载预设与自定义命名模板 (Download Presets & Naming Templates)

*   **现状分析**：高级用户可能有固定的下载偏好，每次手动选择很繁琐。
*   **优化建议**：
    1.  **设置页增加“下载预设”管理**：允许用户创建多个预设，每个预设可以保存一套完整的下载配置（如：格式选择偏好、后期处理选项、甚至自定义的 yt-dlp 原始参数）。
    2.  **新建任务时应用预设**：在“新建下载”Modal 中，提供一个下拉菜单让用户选择使用哪个预设。
    3.  **自定义文件名模板**：在设置页提供一个输入框，允许用户使用 yt-dlp 的[输出模板变量](https://github.com/yt-dlp/yt-dlp#output-template)（如 `%(uploader)s - %(title)s [%(id)s].%(ext)s`）来定义全局的文件名格式。

*   **技术实现补充**：
    *   `config.json` 中增加 `presets` 和 `output_template` 字段。
    *   `ytdlp.rs` 在构建下载命令时，使用 `-o` 参数传入用户自定义的模板。

#### 4. 生态与健壮性 (Ecosystem & Robustness)

##### 建议 6：内置依赖的自我修复与更新

*   **现状分析**：文档已规划了 `yt-dlp` 的热更新，非常棒。但下载任务还强依赖 `ffmpeg` 进行合并。
*   **优化建议**：
    1.  **`ffmpeg` 的捆绑与更新**：同样将 `ffmpeg` 二进制文件视为核心引擎的一部分。在 `updater.rs` 中，不仅检查 `yt-dlp`，也定期检查（或提供手动检查）`ffmpeg` 的更新。可以使用一些知名的 `ffmpeg` 自动构建 Release 源（如 [Gyan.dev](https://www.gyan.dev/ffmpeg/builds/) 或 [BtbN](https://github.com/BtbN/FFmpeg-Builds/releases) 的 API）。
    2.  **依赖健康检查**：在设置页面增加一个“依赖项检查”功能，点击后程序会自动验证 `yt-dlp` 和 `ffmpeg` 是否存在、版本是否可读、是否能正常执行 (`--version`)，并将结果清晰地反馈给用户，方便排查问题。

---


1. youtube，douyin的内容无法下载
2. 希望对流程进行归纳