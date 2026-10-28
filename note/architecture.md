# SoMarkClient CE 架构

## 进程与界面

应用采用 Tauri 2 + React + TypeScript。React 只负责展示、参数编辑和用户操作；上传、队列、重试、凭据、SQLite、下载和文件写入均在 Rust 核心中完成，不需要 Node.js 或 Python sidecar。

主要目录：

- `src/`：多页面工作台、任务状态中心、结果预览和中英文文案。
- `src-tauri/src/api.rs`：SoMark 兼容 HTTP API、流式上传、QPS 限制与下载。
- `src-tauri/src/queue.rs`：并发调度、轮询、重试和恢复状态机。
- `src-tauri/src/store.rs`：SQLite 事务、任务与服务配置、结果元数据。
- `src-tauri/src/credentials.rs`：Windows 凭据管理器与便携会话 Key。
- `src-tauri/src/input.rs`：文件、文件夹、PDF 与剪贴板预检。
- `src-tauri/src/results.rs`：原子保存、ZIP 资源、HTML/TXT 导出与重名保护。
- `src-tauri/src/assets.rs`：按任务授予源文件和结果资源访问权限。
- `src-tauri/src/desktop.rs`：单实例、托盘、关闭窗口后台运行。

## 任务状态

本地状态包括：

- `queued`：等待调度。
- `uploading`：文件流正在上传，界面显示真实上传比例。
- `submissionUnknown`：上传后未确认服务端响应；禁止自动重新提交。
- `waiting`：已有云端任务 ID，正在查询真实云端状态。
- `stopped`：停止本地等待，保留任务 ID 或结果供以后恢复。
- `paused`：不参与调度，可恢复。
- `downloading`：结果已保存，正在写入输出或下载资源。
- `success`：请求的全部输出均已完成。
- `partialSuccess`：解析或转换成功，但部分输出/资源失败，可单独补齐。
- `failed`：任务失败，保留错误类型和重试信息。

异步提交成功后先持久化 `task_id`，再开始轮询。网络查询和资源下载采用分类重试与带抖动的指数退避；首次上传或同步提交遇到不确定网络结果时，不自动重试。

即使本地已经停止或暂停，只要云端响应已到达，也会先保存任务 ID 或结果，再保持用户选择的停止/暂停状态。

## 并发与恢复

默认全局并发和每个服务并发均为 1。只有用户明确确认获得更高额度后，才允许提高服务并发。调度器同时受全局并发、服务并发和服务 QPS 限制。

任务参数、状态、云端 ID、结果路径、输出清单和重试计数存储在 SQLite（WAL 模式）中。启动时：

- 上传中断的任务转为 `submissionUnknown`，不自动上传。
- 未提交任务继续排队。
- 已有云端任务 ID 的任务恢复查询。
- 已保存结果的任务恢复输出下载。
- 超过保留期的终态历史被清理，用户导出的文件不删除。

## 凭据与隐私

安装版使用 Windows 凭据管理器，以 `com.somarkclient.ce` 和服务配置 UUID 保存 Key。数据库仅保存配置及凭据引用。

便携版由 EXE 同目录的 `portable.flag` 识别，数据写入 EXE 同目录的 `data/`，Key 仅保存于内存。重启后需要重新录入。

应用不收集使用遥测。日志只包含应用自身的脱敏事件；诊断包不包含 Key、源文件 URL、签名参数或文档正文。

## 文件与结果

上传前检查官方扩展名、空文件、大小和可读性；PDF 尽可能检查页数、加密和损坏。单批最多 1000 个文件。

每个源文件使用独立输出目录。输出先写 `.part` 临时文件，完成后重命名；重名自动追加序号。ZIP 下载后安全解压，拒绝路径穿越、过多条目和超过 2GB 的解压总量。

PDF/图片原文预览通过 PDF.js 分页或单页画布完成，`bbox` 按 `page_size` 比例叠加。Office 等无法可靠还原本地原版的格式提供源文件打开入口，不伪造坐标叠层。

HTML 导出经过 sanitize，不包含可执行脚本。TXT 优先按 JSON 页面和内容块顺序生成，缺少 JSON 时从 Markdown 提取文本。

## 构建与分发

结果预览使用动态 import，主界面与 PDF/Markdown 模块分包。安装版使用 NSIS x64 当前用户安装；便携 ZIP 包含同一 EXE、许可证和 `portable.flag`。两种产物均不需要安装 Node.js、Python 或 Rust。
