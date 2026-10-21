# 官方 API 调研记录

Google 上传与新增 OneDrive 核验日期：2026-10-21；百度及 Google 其余协议记录沿用 2026-08-30。

本记录只采信官方开放平台、官方文档与官方 SDK。未找到官方依据的能力标为“当前未确认官方 API 支持”，代码不会猜测 URL、参数或授权方式。

## 百度网盘

主要依据：[OAuth 接入指南](https://openauth.baidu.com/doc/doc.html)、[百度网盘开放平台](https://yun.baidu.com/open/platform)、[百度网盘官方 Go SDK](https://github.com/baidu-netdisk/baidu-drive-sdk-go)。

| 能力 | 当前确认结果 | 当前实现 |
| --- | --- | --- |
| OAuth | OAuth 2.0；官方 SDK 提供授权码和设备码模式 | 使用设备码并在浏览器确认 |
| Token Refresh | 官方支持使用 `refresh_token` 获取新 Token | 过期前自动刷新，失败后要求重新授权 |
| Scope | 官方 SDK 设备码请求为 `basic,netdisk` | 使用相同 Scope |
| 上传 | `precreate → superfile2 分片 → create` | 已实现 |
| 分片 | 官方 SDK 默认 4 MiB | 一次只读取一个 4 MiB 分片 |
| 秒传 | `precreate` 返回秒传结果时可跳过分片 | 已实现 |
| 断点续传 | `precreate` 返回缺失分片，`uploadid` 可继续完成 | SQLite 保存 uploadid、缺失与完成分片 |
| 文件列表 | 官方 SDK 提供 ListDir | Adapter 能力保留，上传链路暂未调用 |
| 创建目录 | 官方 SDK 提供 Mkdir | 当前使用已配置的应用目录 |
| 创建分享 | 个人开放 API 当前未确认官方支持 | 显示真实限制，不伪造链接 |
| 分享查询 | 当前未确认个人官方 API 支持 | 不实现 |
| 分享取消 | 当前未确认个人官方 API 支持 | 不实现 |
| 限流 | 官方 SDK 列出 errno `31034` 为频率超限；未确认固定通用 QPS | 有限指数退避，到达上限后停止 |
| 单文件大小限制 | 当前资料未确认统一固定值 | 不写死，显示服务端真实错误 |

开放平台官网展示企业文件分享场景，不代表个人开放 API 已取得相同接口。当前版本因此仅返回官方能力限制说明，等待官方明确开放后再在 `DriveAdapter` 中增加实现。

## Google Drive

主要依据：[桌面应用 OAuth 2.0](https://developers.google.com/identity/protocols/oauth2/native-app)、[Drive API Scope](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)、[文件上传](https://developers.google.com/workspace/drive/api/guides/manage-uploads)、[创建文件夹](https://developers.google.com/workspace/drive/api/guides/folder)、[分享管理](https://developers.google.com/workspace/drive/api/guides/manage-sharing)、[Permission 资源](https://developers.google.com/workspace/drive/api/reference/rest/v3/permissions)与[用量限制](https://developers.google.com/workspace/drive/api/guides/limits)。

| 能力 | 当前确认结果 | 当前实现 |
| --- | --- | --- |
| OAuth | Installed/Desktop app 打开系统浏览器；Windows 支持随机 loopback IP 回调 | `127.0.0.1:随机端口`，单次回调 |
| PKCE / CSRF | 官方支持并推荐 S256 PKCE；要求客户端使用 `state` 防 CSRF | 每次授权生成独立 verifier、challenge 与 state |
| Token Refresh | Token endpoint 为 `https://oauth2.googleapis.com/token`；offline access 返回 Refresh Token | 提前 120 秒刷新，失败后要求重新授权 |
| Scope | `drive.file` 为非敏感的最小权限，可管理应用创建或用户授权的文件 | 只申请 `https://www.googleapis.com/auth/drive.file` |
| 上传 | `files.create?uploadType=resumable` 返回 `Location` session URI | 非空文件统一使用 resumable upload |
| 分片 | 除最后一片外必须为 256 KiB 整数倍 | 使用 8 MiB 分片，一次只读一片 |
| 断点续传 | 空 PUT 与 `Content-Range: bytes */total` 查询状态；`308` 的 `Range` 是真实已接收范围；session 一周后失效 | SQLite 保存 session URI，恢复时以服务器范围为准；六天后主动新建 session |
| 空文件 | 小文件可使用 multipart upload | 空文件使用官方 multipart/related 上传 |
| 秒传 | 当前未确认 Drive API 提供按本地 Hash 跳过上传的官方能力 | 不伪造秒传；只复用本地历史中已上传的 fileId |
| 文件列表 | `files.list` 支持 `q` 查询 | 用于按父目录与名称查找应用可见目录 |
| 创建目录 | folder 是 MIME type 为 `application/vnd.google-apps.folder` 的 file | 按默认路径逐层查询或创建 |
| 创建分享 | `permissions.create` 可创建 `type=anyone, role=reader` | 已实现，并读取 `files.webViewLink` |
| 分享查询 | `permissions.list/get` 官方支持 | 创建前用 `permissions.list` 避免重复公开权限 |
| 分享取消 | `permissions.delete` 官方支持 | Adapter 接口预留，MVP 尚未提供取消分享 UI |
| 提取码 | Google Drive 官方分享 Permission 没有提取码字段 | 不显示或输出提取码 |
| 分享有效期 | `expirationTime` 只能设置在 user/group 权限，不能用于 anyone | 公开链接保存为无到期时间，不伪造设置值 |
| 限流 | 403 rate limit、429 与 5xx 应使用有限指数退避 | 有上限退避；永久 403 直接显示权限或组织策略错误 |
| 单文件大小限制 | `files.create` 当前官方上限为 5,120 GB；账号空间和每日上传限制仍适用 | 不预分配整文件内存，服务端限制原样报错 |

Google Workspace 管理员可以禁止公开分享。此时上传本身可能已经成功，但创建 `anyone/reader` Permission 会失败；任务管理器会保留 fileId 和远端路径，用户重试 Google 时只重新执行分享步骤。

OAuth Desktop app 无法把 Client Secret 当作真正机密，因此官方协议把 `client_secret` 标为可选。本项目兼容凭据 JSON 中包含 Secret 的常见桌面客户端，同时仍把它放入 Windows Credential Manager，绝不写入仓库、SQLite 或日志。

## 已移除的平台

夸克网盘支持已从当前应用运行时中移除。程序不再检测或安装 Skill，不再调用 WSL、CLI、Cookie 或任何夸克接口，也不再暴露夸克授权、目录、任务、历史或复制入口。

旧数据库可能保留 `quark_` 前缀列，只用于无损升级兼容。当前代码不读取、展示或更新这些列。外部环境中已经由用户安装的 Skill 不属于应用数据，升级或卸载本工具都不会擅自删除它。

## 后续接入原则

接入或升级平台前必须重新核验官方文档。只有官方资料明确给出端点、字段、授权约束和桌面应用使用条件后才能增加实现。平台能力必须进入独立 `BaiduDriveAdapter` 或 `GoogleDriveAdapter`，不得把私有调用散落在任务管理器或 Vue 页面中。

## Microsoft OneDrive（2026-10-21）

依据：[设备码 OAuth](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-code)、[上传会话](https://learn.microsoft.com/en-us/graph/api/driveitem-createuploadsession?view=graph-rest-1.0)、[创建目录](https://learn.microsoft.com/en-us/graph/api/driveitem-post-children?view=graph-rest-1.0)、[创建分享链接](https://learn.microsoft.com/en-us/graph/api/driveitem-createlink?view=graph-rest-1.0)。本次采用现有 Rust reqwest 和统一 Adapter 接口，不增加 SDK 依赖。

| 能力 | 官方契约与实现 |
| --- | --- |
| OAuth | public client，common 租户设备码；个人和工作/学校账号；设备码只留在后端内存，前端持有随机句柄 |
| Refresh | offline_access；安全存储 Refresh Token，串行刷新及轮换 |
| Scope | Graph Files.ReadWrite 委托权限；此权限允许读写当前用户文件，比 Google drive.file 范围更广 |
| 上传 | createUploadSession，5 MiB 分片为 320 KiB 整数倍；空文件 content PUT；同名冲突 fail |
| 断点 | GET uploadUrl，解析 nextExpectedRanges；仅明确 404/410 且结果不存在时新建，网络错误保留原会话 |
| 完成恢复 | 文件名追加固定任务 ID；会话绑定 drive ID，完成响应丢失时读取对应项确认大小，避免重传 |
| URL | 预授权上传 URL 不发送 Graph Authorization，禁止重定向，HTTPS 微软上传域名白名单；不记日志 |
| 目录 | 按路径查询并创建 folder，409 后重新查询，拒绝同名普通文件 |
| 分享 | createLink view/anonymous，复用已有链接；expirationDateTime 按服务端返回展示；组织策略禁止时保留文件 |
| 限流 | 429、5xx 和网络失败进入任务级有限退避；401 刷新一次，永久 4xx 不自动重试 |
| 大小 | 分片缓冲固定 5 MiB；最终上限由微软账号空间及服务限制决定，不伪造秒传 |
| 边界 | 全球云端点；独立主权云不支持。无真实凭据时只能验收本地测试、构建和模拟页面 |

Google 本次保留现有 Desktop OAuth、drive.file、8 MiB resumable 和 anyone/reader 分享，新增跨平台分享重试复用修复及回归检查。

## 绑定引导复核（2026-10-21）

页面内的首次设置步骤依据 [Google Drive 快速入门](https://developers.google.com/workspace/drive/api/quickstart/python?hl=zh-cn)、[Microsoft 应用注册](https://learn.microsoft.com/zh-cn/entra/identity-platform/quickstart-register-app)、[微软认证流限制](https://learn.microsoft.com/en-us/entra/identity-platform/msal-authentication-flows)和前述百度 OAuth 接入指南。Google 选择 Desktop app、下载配置并为测试应用添加测试用户；微软个人账号需选择包含个人账号的应用类型，设备码需要 public client。普通用户登录时无需了解这些参数；缺少应用配置时仍需发布者或用户先完成首次设置。
