# 官方 API 调研记录

核验日期：2026-08-30。

本记录只采信官方开放平台、官方文档、官方 SDK 或官方维护仓库。未找到官方依据的能力标为“当前未确认官方 API 支持”，代码不会猜测 URL、参数或授权方式。

## 百度网盘

主要依据：[OAuth 接入指南](https://openauth.baidu.com/doc/doc.html)、[百度网盘开放平台](https://yun.baidu.com/open/platform)、[百度网盘官方 Go SDK](https://github.com/baidu-netdisk/baidu-drive-sdk-go)。

| 能力 | 当前确认结果 | MVP 处理 |
| --- | --- | --- |
| OAuth | OAuth 2.0；官方 SDK 提供授权码和设备码模式 | 使用设备码，浏览器完成授权 |
| Token Refresh | 官方支持 `refresh_token` 换取新 Token | 过期前自动刷新，失败后要求重新授权 |
| Scope | 官方 SDK 设备码请求为 `basic,netdisk` | 使用相同 Scope |
| 上传 | `precreate → superfile2 分片 → create` | 已实现 |
| 分片 | 官方 SDK 默认 4 MiB | 每次只读取一个 4 MiB 分片 |
| 秒传 | precreate 返回秒传结果时可跳过分片 | 已实现 |
| 断点续传 | precreate 返回缺失分片；uploadid 可继续完成分片 | SQLite 保存 uploadid、缺失与完成分片 |
| 文件列表 | 官方 SDK 提供 ListDir | Adapter 能力保留；MVP 上传链路暂未需要 |
| 创建目录 | 官方 SDK 提供 Mkdir | 当前文件上传使用已配置应用目录；目录预创建增强待后续版本 |
| 创建分享 | 个人开放 API 当前未确认官方支持 | 明确显示上传成功但不能自动分享 |
| 分享查询 | 当前未确认个人官方 API 支持 | 不实现、不伪造 |
| 分享取消 | 当前未确认个人官方 API 支持 | 不实现、不伪造 |
| 限流 | 官方 SDK 列出 errno `31034` 为频率超限；未确认固定通用 QPS | 有限指数退避，达到上限停止 |
| 单文件大小限制 | 当前资料未确认统一固定值 | 不写死，展示服务端真实错误 |

开放平台官网展示企业文件分享场景，不代表个人开放 API 已获得同等分享接口。MVP 因此把分享放在统一 Adapter 能力层，待获得官方企业权限或官方发布个人分享接口后再替换实现。

## 夸克网盘

主要依据：[夸克网盘官方 Skill 仓库](https://github.com/quark-clouddrive/quarkclouddrive_offical)及仓库内 v1.0.15 的 `SKILL.md`、`references/auth.md`、`file-upload.md`、`file-share.md` 与 `file-ops.md`。

| 能力 | 当前确认结果 | MVP 处理 |
| --- | --- | --- |
| OAuth | 官方 CLI `login` 启动浏览器 OAuth | 通过 WSL 调用官方 CLI |
| Token Refresh | 由官方 CLI/SDK 维护登录态；公开文档未要求应用直接处理 Token | 不读取 Token，不保存 Cookie |
| 上传 | 官方 CLI `upload` 支持文件与文件夹 | 任务管理器逐文件调用，保留独立状态 |
| 真实进度 | CLI 输出 NDJSON `current/total/percent` | 逐行解析并显示 |
| 秒传 | 结果返回 `instantUpload` | 已识别并记录 |
| 断点续传 | `upload list`、`upload resume --record-id` | 保存 recordId 并调用 resume |
| 文件列表/搜索 | 官方 CLI 提供搜索与文件浏览能力 | MVP 上传链路暂未调用 |
| 创建目录 | `create-folder --dir-path`，同名创建幂等 | 上传前创建或取得目标目录 FID |
| 创建分享 | `share <FID...>` | 已实现公开链接 |
| 分享有效期 | 1=永久，2=1天，3=7天，4=30天，5=60天，6=100天，7=180天 | 设置值映射到官方枚举 |
| 分享查询 | `share-detail --url` | 统一 Adapter 能力已保留；MVP 直接使用创建结果 |
| 分享取消 | 当前公开 Skill 文档未确认提供面向已创建分享的取消命令 | 不实现、不猜测 |
| 限流规则 | 当前文档未确认固定通用阈值 | 不写死，失败有限重试 |
| 单文件大小限制 | 当前文档未确认统一固定值 | 不写死，展示服务端真实错误 |
| Windows 支持 | 官方表格：Windows WSL amd64 支持，Windows 原生不支持 | 使用 `wsl.exe` 调用脚本；缺失时 Adapter 不可用 |

官方 v1.0.15 入口是 Skill 目录内的 `node scripts/quark-drive.cjs`，安装脚本会清理旧版全局 `quarkclouddrive` 命令。本项目因此使用可配置脚本路径，不依赖旧命令名。

## 后续接入原则

重新接入或升级平台前，先复核官方仓库最新版本和开放平台文档。只有在官方资料明确给出端点、字段和授权约束后才能增加实现。新增平台能力必须进入 `DriveAdapter`，不能把私有调用散落在任务管理器或 Vue 页面中。
