# SoMarkClient CE

SoMarkClient CE 是一个第三方 SoMark 文档解析桌面客户端，使用 Tauri、React 和 Rust 构建，支持 Windows 10/11 x64。

用户只需配置 API Key 和待转换文档，即可通过 SoMark API 完成解析、结果预览和文件导出。项目重点关注可视化任务管理与断网、重启后的任务恢复能力。

## 主要功能

- 支持本地文件、文件夹、公开或预签名 URL、剪贴板图片。
- 支持 PDF、图片、Word、PPT、Excel 等 SoMark 官方文档格式。
- 输出 Markdown、JSON、ZIP，并从 ZIP 恢复 Markdown 和本地图片。
- 支持 PDF 转 Word/PPT，以及 HTML、纯文本导出。
- 提供同步/异步任务、真实上传进度、云端状态和历史记录。
- 支持 Markdown/JSON 预览、目录导航、原文对照和坐标高亮。
- 使用 SQLite 持久化队列，支持断网恢复、重启恢复和部分成功重试。
- 提供中英文界面、深浅主题、托盘通知和脱敏诊断日志。

## 下载与使用

从 [GitHub Releases](https://github.com/hershate/SoMarkClient-CE/releases/latest) 下载：

- `SoMarkClient.CE_<版本号>_x64-setup.exe`：NSIS 安装包。
- `SoMarkClient-CE-<版本号>-portable.zip`：免安装便携版。

使用步骤：

1. 打开 **服务与额度**，添加 API Key。
2. 默认 Base URL 为 `https://somark.cn/api/v1`，请按 Key 的来源平台或私有部署地址调整。
3. 验证 Key 和剩余额度。
4. 在 **新建任务** 中选择文档、输出格式和保存目录，然后开始处理。
5. 在 **任务中心** 查看进度，完成后预览或打开输出文件。

安装版的 API Key 保存在 Windows 凭据管理器中；便携版仅在本次运行期间保留 Key，不会持久保存。应用不收集使用遥测。

## 本地构建

需要 Node.js、pnpm、Rust（MSVC）和 Windows C++ Build Tools。

```bash
pnpm install
pnpm tauri dev
```

执行检查与构建：

```bash
pnpm build
pnpm tauri build
```

生成便携版：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1
```

具体工具链路径和 NSIS 离线缓存配置见 [构建说明](note/build.md)。

## 项目文档

- [架构说明](note/architecture.md)
- [SoMark API 兼容说明](note/api-compatibility.md)
- [版本记录](note/release)
- [SoMark 官方文档](https://docs.somark.cn/documentation/sdk)

## 许可证

本项目基于 [Apache License 2.0](LICENSE) 开源。SoMark 名称和相关服务属于其相关权利人，本项目不是 SoMark 官方客户端。
