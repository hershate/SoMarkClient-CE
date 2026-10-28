# SoMark API 兼容说明

实现以以下官方文档为准：

- SDK：https://docs.somark.ai/documentation/sdk
- CLI & SDK 配置：https://docs.somark.ai/cli-sdk/usage
- API 概览：https://docs.somark.ai/api-reference/index
- 同步解析：https://docs.somark.ai/api-reference/endpoint/sync
- 异步提交：https://docs.somark.ai/api-reference/endpoint/async-submit
- 异步查询：https://docs.somark.ai/api-reference/endpoint/async-check
- PDF 转 Office：https://docs.somark.ai/api-reference/endpoint/pdf2office-sync
- Office 异步提交：https://docs.somark.ai/api-reference/endpoint/pdf2office-async-submit
- Office 异步查询：https://docs.somark.ai/api-reference/endpoint/pdf2office-async-check
- 用量：https://docs.somark.ai/api-reference/endpoint/usage-query
- 错误码：https://docs.somark.ai/api-reference/errors
- 图片持久化：https://docs.somark.ai/documentation/image-persistence

## 使用的接口

| 功能 | 方法与路径 |
| --- | --- |
| 同步解析 | `POST /parse/sync` |
| 异步解析提交 | `POST /parse/async` |
| 异步解析查询 | `POST /parse/async_check` |
| 同步 Office 转换 | `POST /convert/sync` |
| 异步 Office 提交 | `POST /convert/async` |
| 异步 Office 查询 | `POST /convert/async_check` |
| Key 验证与用量 | `POST /usage` |

不使用计划于 2026-12-31 停用的 `/extract/*` 旧路径。

## 字段约定

- API Key 通过 multipart 请求体中的 `api_key` 发送，不写入日志。
- `file` 与 `file_url` 严格互斥。
- 多个 `output_formats` 作为重复 multipart 字段发送，值为 `markdown`、`json`、`zip`。
- ZIP 与 `element_formats.image=file` 必须同时选择。
- `.cn` 实际要求 ZIP 不能与 Markdown 同时请求（ZIP 只能单独使用或与 JSON 组合）。因此同时选择 Markdown/JSON/ZIP 时统一请求 `json + zip`，再从 ZIP 中恢复 Markdown，用户最终得到的输出格式不变。
- `feature_config` 显式序列化为官方蛇形字段，如 `enable_text_cross_page`、`enable_title_level_recognition`，不直接发送前端使用的驼峰字段。
- API 响应中的 `task_id`、`page_num`、`remaining_paid_pages` 等蛇形字段通过反序列化别名映射到前端驼峰模型。
- 错误响应的 `data` 可能是空字符串而非对象，因此先解析 `code`/`message`，只有成功时才将 `data` 解码为具体响应模型。

## 限制与状态

正式服务默认按单文件 200MB、最多 300 页、QPS 4 处理。服务配置可调整限制以适配私有化或兼容中转，但不会因此改变官方服务端限制。

解析轮询状态为 `queuing`、`processing`、`success`、`failed`。Office 状态为 `PARSING`、`QUEUING`、`CONVERTING`、`SUCCESS`、`FAILED`。默认每 4 秒轮询一次。实际服务可能返回大写 `SUCCESS`，客户端按大小写不敏感识别成功状态并进入下载。

接口不提供真实解析百分比。界面只展示上传比例、真实云端阶段和耗时，不显示伪造的云端进度。

已查阅的官方接口列表未提供服务端取消任务接口。因此“停止等待”只停止本地轮询，不宣称取消云端任务。

## 资源有效期

远程图片 URL 有效期为 30 天。默认同时请求 ZIP 并下载、解压图片资源，预览优先使用本地资源。

Office 下载地址为短期签名 URL。异步任务可通过已有 `task_id` 再次查询新的下载地址。

## 直连与中转

每个命名服务配置都有独立 Base URL、Key、代理和限制。官方地址、私有化地址和兼容中转使用同一协议适配器；中转服务必须实现相同的接口、字段和响应约定。首版不支持自定义的中转响应协议。

默认服务地址统一为 `https://somark.cn/api/v1`，避免误用 `.cn` Key 请求 `.ai` 服务。若 Key 明确创建于其他平台，仍可在服务配置中手动改为对应地址。实际验证中，`somark.cn` 创建的 Key 在 `.ai` 返回 `1107`，切换到 `.cn` 后用量查询返回 `code: 0`。
