# 双盘发货助手

双盘发货助手是一款 Windows 本地桌面工具。当前版本仅接入百度网盘：把单个文件、多个文件或文件夹拖入窗口后，程序会自动计算 SHA-256、上传百度网盘、尝试获取官方分享结果，并保存任务与历史记录。

项目使用 Tauri 2、Vue 3、TypeScript、Vite、Rust 与 SQLite。它不会伪造分享链接，也不会使用 Cookie、抓包或逆向私有接口。平台能力边界以 2026-08-30 核验的官方资料为准，详见 [API 调研记录](docs/API_RESEARCH.md)。

## 当前已实现

支持文件与文件夹拖放，文件夹会递归展开为独立文件任务。

使用流式 SHA-256 和文件大小识别重复文件，不把完整文件读入内存。已有有效分享时直接显示历史结果；已有百度远端文件但分享失效时只重新尝试分享；旧记录中没有百度结果时会正常补传百度网盘。

百度使用官方 OAuth 2.0 设备码授权，Token 自动刷新，App Key、Secret Key、Access Token 与 Refresh Token 保存到 Windows Credential Manager。

百度上传采用官方 `precreate → superfile2 分片 → create` 流程，按 4 MiB 分片流式读取，支持秒传判断、缺失分片续传、真实字节进度、暂停、取消和有限次数重试。

SQLite 保存任务、恢复信息、设置与历史记录。程序异常退出后，未完成任务会恢复为暂停状态。

历史页支持文件名搜索和再次复制百度分享文案。日志使用 JSONL，并遮盖 Token、Secret、Cookie 与 Authorization 等敏感信息。

夸克网盘的运行时、授权、上传、分享、设置和界面支持已从当前版本移除。升级不会删除旧数据库中的兼容列，但程序不再读取、展示或更新夸克数据，也不会安装或调用夸克 Skill、WSL 或 CLI。

## 已确认限制

百度个人开放 API 当前未确认提供创建公开分享链接的能力。文件上传成功后，界面会如实显示“文件上传成功，当前百度个人开放 API 文档未确认支持自动创建公开分享”，不会构造假链接。

百度开放平台官网存在企业文件分享方案，但本项目不把企业能力冒充为个人开放接口。账号、配额和平台服务端策略也可能影响上传大小、秒传与限流结果；未被官方统一确认的数字不会写死在客户端。

## 本地开发

环境要求为 Windows 10/11、WebView2 Runtime、Node.js 20 或更高版本、Rust stable，以及 Visual Studio 2022 Build Tools 的“使用 C++ 的桌面开发”工作负载。

```powershell
npm.cmd install
npm.cmd run tauri dev
```

仅启动前端页面：

```powershell
npm.cmd run dev
```

运行测试与前端生产构建：

```powershell
npm.cmd test
npm.cmd run build
cargo test --manifest-path src-tauri/Cargo.toml
```

如果普通 PowerShell 未加载 MSVC 环境，请使用 Visual Studio Developer Command Prompt，或先运行 `VsDevCmd.bat -arch=x64 -host_arch=x64`。

## Windows 编译

```powershell
npm.cmd run tauri build -- --bundles nsis
```

安装包生成到 `src-tauri/target/release/bundle/nsis/`，独立可执行文件生成到 `src-tauri/target/release/`。`src-tauri/target` 不应提交到 Git。

## 百度开放平台申请

在[百度网盘开放平台](https://yun.baidu.com/open/platform)创建应用，按官方要求完成应用资料、隐私政策与能力申请，取得 App Key 和 Secret Key。首次运行后进入“设置”，填写凭据并点击“绑定百度网盘”。程序会打开官方设备码授权页，授权完成后凭据和 Token 只保存在系统安全存储中。

当前 OAuth Scope 为官方 SDK 使用的 `basic,netdisk`。设备码流程不要求桌面应用监听回调端口。若以后切换授权码模式，必须按[百度 OAuth 接入指南](https://openauth.baidu.com/doc/doc.html)登记完全一致的回调地址。

百度默认目录必须位于开放平台分配给应用的 `/apps/应用目录/` 下。项目初始值是 `/apps/双盘发货助手/自动发货/`；如果平台分配的应用目录名不同，必须在设置页改成实际目录。

## 环境变量

复制 `.env.example` 为 `.env` 仅用于本机开发，不要提交 `.env`。

`BAIDU_APP_KEY` 和 `BAIDU_SECRET_KEY` 是开发期可选初值。生产使用建议在设置页写入 Windows Credential Manager。

## OAuth 回调

当前设备码授权没有本地 OAuth 回调地址。应用通过官方设备码接口取得授权码，浏览器完成用户确认，桌面端按官方建议的间隔有限轮询结果。

## 数据库与日志

默认数据目录：

```text
%APPDATA%\com.dualdrive.shipper\
```

数据库文件是 `dual-drive-assistant.db`，日志位于同目录的 `logs` 子目录。SQLite 使用 WAL 模式。设置页提供“打开日志目录”按钮。

为兼容旧版本，数据库表中可能仍存在名称以 `quark_` 开头的历史列。这些列不会被当前版本读取、显示或更新，保留它们只是为了避免升级时执行破坏性数据迁移。

## 项目目录

```text
src/
  components/        Vue 拖放、任务、结果、历史和设置组件
  services/          Tauri 命令调用封装
  types/             TypeScript 结构定义
  utils/             格式化与测试
src-tauri/src/
  adapters/baidu.rs  BaiduDriveAdapter
  auth.rs            百度 OAuth 与 Token Refresh
  database.rs        SQLite 任务、历史、设置与兼容迁移
  task_manager.rs    Hash、去重、并发、暂停、取消和恢复
  share_manager.rs   百度分享文案模板
  secret_store.rs    Windows Credential Manager
  logging.rs         JSONL 日志与脱敏
  commands.rs        Tauri 命令边界
docs/API_RESEARCH.md 官方 API 核验记录
.codex/skills/       项目维护经验 Skill
```

## 常见错误

“百度 Token 已失效或权限不足”：程序会先尝试 Refresh；仍失败时，在设置页清除授权后重新绑定，并确认应用已获得网盘权限。

“百度上传路径不合法”：把默认目录改为开放平台实际分配的 `/apps/应用目录/`，不要直接上传到用户网盘任意根目录。

“HTTP 429 / errno 31034”：平台触发限流。程序会有限指数退避；达到设置的重试上限后保留为可重试状态。

“link.exe / Windows SDK 未找到”：安装 Visual Studio 2022 Build Tools 的 C++ 桌面工作负载，然后在 Developer Command Prompt 中重新构建。

“上传完成但没有分享链接”：百度个人开放 API 当前未确认支持公开分享创建，这是已知官方能力边界，不是客户端伪造失败。

## 隐私与安全

隐私说明见 [PRIVACY.md](PRIVACY.md)。`.gitignore` 已排除 `.env`、Token、本地数据库、日志、构建目录和本地工作数据。不要把真实账号、Client Secret、Cookie 或测试文件提交到仓库。

## 验收边界

源码、单元测试、前端生产构建和 Windows 安装包可以在无账号环境下验证。真实百度上传、秒传、断点续传和账号配额行为必须使用用户自己的已审核开放平台应用与授权账号完成，不在仓库中提供公共测试账号或凭据。
