# 双盘发货助手

[English](README.en.md)

## 独立浏览器版 0.1.0

无需安装桌面助手。[下载浏览器扩展](https://github.com/guoji1hao/drive-delivery/releases/tag/browser-v0.1.0)，解压后在 Chrome / Edge 扩展管理页打开开发者模式，点“加载已解压的扩展程序”，选中解压目录，再点工具栏图标。详细图文案步骤见 [安装与绑定说明](browser-extension/INSTALL.md)。

支持中英文、三个官方网盘适配模块、点击 + 选文件、文件夹上传、暂停后续队列、失败重试和本地结果历史。Google 授权仅支持 Chrome。首次需要配置自己的平台应用；当前没有内置公共 OAuth 配置，尚未上架扩展商店。上传时保留页面，关闭或刷新会丢失未完成队列；浏览器版不提供重启续传。公开分享默认关闭。

桌面版和浏览器版独立运行，下面的 Windows 凭据管理、SQLite、重启续传说明仅适用于桌面版。浏览器版详细限制与验收边界见 [验证说明](docs/BROWSER_VERIFICATION.md)。

双盘发货助手是一款 Windows 本地桌面工具。把单个文件、多个文件或文件夹拖入窗口后，程序会调用已启用的网盘插件上传，支持百度网盘、Google Drive 与 OneDrive 官方 API，按平台独立显示真实进度，自动尝试创建分享，并把可用结果整理为一键复制的发货文案。

项目使用 Tauri 2、Vue 3、TypeScript、Vite、Rust 与 SQLite。它不会使用 Cookie、账号密码、抓包或逆向私有接口，也不会伪造平台能力。当前能力边界以 2026-10-21 核验的官方资料为准，详见 [API 调研记录](docs/API_RESEARCH.md)。

## 第一次怎么绑定（直接照着做）

先在“绑定网盘 → 网盘插件”中启用要用的网盘，其他的点“停用”。删掉的插件可一键“安装并启用”，也能导入 `plugins/` 中的插件文件。现有版本升级默认保留三个插件开启。详见[插件说明](docs/DRIVE_PLUGINS.md)。

顶部可随时切换简体中文和 English，下次打开会记住。界面、绑定教程、默认分享文案及安装程序支持两种语言；用户自定义模板、文件名、目录保持原文，平台原始错误诊断可能保留原始语言。

打开软件顶部的“绑定网盘”，找到你想用的网盘，点“绑定”。程序会打开网盘自己的网页，你只要登录账号并点同意，然后回到软件；显示“已绑定”就完成了。以后打开会记住，不用每天再绑定。

百度和微软如果要求验证码，软件会显示大号验证码并尝试自动复制。到官方网页按 Ctrl+V 粘贴即可；没复制成功就点“复制验证码”。网页误关了可点“重新打开登录页”。Google 通常直接选择账号并同意。

如果第一次点绑定出现“首次设置”，说明这份软件还没配置对应平台的应用。通常应由软件提供者准备。页面里已经写好中文操作步骤，并有官方入口；Google 可直接导入官网下载的桌面应用 JSON 文件，省去手工抄写。保存后再绑定或换账号都不需要重新填写。

网盘密码只在官方网页输入，不要把它填到“应用编号”或“应用密钥”里。真正面向普通用户的一键登录需要软件提供者先完成平台应用注册、发布和必要审核；当前仓库没有内置可供所有人使用的公共应用凭据，不能把尚未配置的安装包说成开箱即用。

上传位置、重试次数等选项放在“上传位置和其他设置（一般不用改）”中。绑定页切到其他页面时会继续等待官方登录结果。

## 添加文件和暂停队列（0.4.0）

点首页的“+”或“选择文件”，可以一次选多个文件。要传整个文件夹，点“上传整个文件夹”后选中它即可；也可以直接拖进窗口。

上传会保留选中的顶层文件夹、内部子目录和空文件夹。队列按文件显示进度，目录任务显示“文件夹”。OneDrive 仍会给文件名追加任务编号以防止覆盖；文件夹上传不会复用其他位置的同内容历史结果，避免漏掉某个目录里的文件。

点“暂停队列”后，排队项目停止开始，正在处理的文件继续完成；期间仍能添加文件。点“继续队列”按顺序处理。文件上的暂停仍只针对该文件的指定网盘。程序关闭后，未完成任务恢复为暂停，需在对应网盘上继续；队列开关只控制当前运行会话。

多个添加批次和手动重试共用同一并发限制。新任务在添加时固定上传目标和日期目录，暂停到第二天或修改默认目录不会把同批文件夹拆散。

## 当前已实现

支持文件选择、文件夹选择与拖放，文件夹保留目录层级并递归生成文件和空目录任务。百度、Google 和 OneDrive 在同一文件内并行运行；任一平台失败不会取消另一平台，暂停、取消和重试也按平台独立执行。

使用流式 SHA-256 与文件大小识别重复文件，不把完整文件读入内存。历史中已有有效分享时直接复用；链接失效时只重新创建分享；某个平台缺少上传记录时只补传该平台。

百度使用官方 OAuth 2.0 设备码授权。上传采用官方 `precreate → superfile2 分片 → create` 流程，按 4 MiB 分片流式读取，支持秒传判断、缺失分片续传、真实字节进度和有限次数退避重试。

Google 使用官方桌面 OAuth 2.0，打开系统浏览器并监听随机 `127.0.0.1` 回调端口，使用 PKCE 与 `state` 防护，只申请 `drive.file` 最小权限。上传采用 Drive API v3 resumable upload，按 8 MiB 分片流式读取；SQLite 保存 session URI 与服务器确认的字节位置，恢复时通过 `308 Range` 继续。上传完成后创建 `anyone/reader` 权限并读取 `webViewLink`，Google 分享没有提取码和到期时间。

OAuth 应用凭据、Access Token 与 Refresh Token 保存到 Windows Credential Manager。Token 过期前自动刷新，刷新失败后才提示重新授权。

SQLite 保存任务、断点、设置与历史记录。程序异常退出后，未完成任务会恢复为暂停状态。历史页支持按文件名搜索，并可复制百度、Google、OneDrive 或全部分享文案。

本地日志使用 JSONL，只记录必要的任务和错误信息，并遮盖 Token、Secret、Cookie 与 Authorization 等敏感字段。设置页可直接打开日志目录。

夸克网盘支持仍保持移除。升级不会删除旧数据库中的兼容列，但程序不会读取、展示或更新夸克数据，也不会安装或调用夸克 Skill、WSL 或 CLI。

## 已确认限制

百度个人开放 API 当前未确认提供创建公开分享链接的能力。文件上传成功后，界面会如实显示“文件上传成功，当前百度个人开放 API 文档未确认支持自动创建公开分享”，不会构造假链接。

Google 的 `anyone` 公开权限不支持设置 `expirationTime`，因此设置中的分享有效期不会强行应用到 Google。Google Workspace 管理员也可能禁止组织账号创建公开链接，此时程序保留已上传文件并把分享步骤标记为可重试。

Google `drive.file` 只允许应用管理自己创建或由用户明确授权给应用的文件。它不会授予扫描整个网盘的权限，也不支持按内容 Hash 秒传；本项目的重复识别来自本地 SQLite，命中后避免再次上传。

## 本地开发

环境要求为 Windows 10/11、WebView2 Runtime、Node.js 20 或更高版本、Rust stable，以及 Visual Studio 2022 Build Tools 的“使用 C++ 的桌面开发”工作负载。

```powershell
npm.cmd install
npm.cmd run tauri dev
```

只启动前端页面：

```powershell
npm.cmd run dev
```

运行测试与前端生产构建：

```powershell
npm.cmd test
npm.cmd run build
cargo test --release --manifest-path src-tauri/Cargo.toml
```

如果普通 PowerShell 未加载 MSVC 环境，请使用 Visual Studio Developer Command Prompt，或先运行 `VsDevCmd.bat -arch=x64 -host_arch=x64`。

## Windows 编译

```powershell
npm.cmd run tauri build -- --bundles nsis
```

安装包生成到 `src-tauri/target/release/bundle/nsis/`，独立可执行文件生成到 `src-tauri/target/release/`。`src-tauri/target` 不应提交到 Git。

## 百度开放平台申请

在[百度网盘开放平台](https://yun.baidu.com/open/platform)创建应用，按官方要求完成应用资料、隐私政策与能力申请，取得 App Key 和 Secret Key。首次运行后进入“设置”，填写凭据并点击“绑定百度网盘”。程序会打开官方设备码授权页。

当前 OAuth Scope 为官方 SDK 使用的 `basic,netdisk`。设备码流程不要求桌面应用监听回调端口。百度默认目录必须位于开放平台分配的 `/apps/应用目录/` 下；项目初始值是 `/apps/双盘发货助手/自动发货/`，如果平台分配的应用目录名不同，必须改成实际目录。

## Google Drive API 接入

打开 [Google Cloud Console](https://console.cloud.google.com/)，创建或选择项目，在 API Library 启用 Google Drive API，然后配置 OAuth consent screen。测试状态的外部应用需要把实际登录账号加入测试用户。

进入 Google Auth Platform 的 Clients 页面，创建类型为 `Desktop app` 的 OAuth 客户端。桌面客户端使用系统浏览器和随机 loopback IP 回调，不需要手工登记某个固定文件路径或固定端口。把下载的凭据 JSON 中 `client_id` 填入设置页；若 JSON 中包含 `client_secret`，一并填写。程序不会把这些值写入仓库或 SQLite。

授权时只请求 `https://www.googleapis.com/auth/drive.file`。首次同意后会获得 Refresh Token；以后程序自动刷新。若出现 `invalid_grant`，在设置页清除 Google 授权后重新绑定。

## Microsoft OneDrive 接入

在 [Microsoft Entra 应用注册](https://entra.microsoft.com/)中创建应用，支持的账号类型选择“任何组织目录中的账号和个人 Microsoft 账号”。在 Authentication 的高级设置中允许公共客户端流；Microsoft Graph 使用委托权限 `Files.ReadWrite`，并请求 `offline_access` 以自动刷新。只需 Application (client) ID，无需创建 Client Secret，也无需配置回调地址。

在设置页填写 Client ID，点击“绑定 Microsoft OneDrive”，在微软官方页面输入界面显示的设备码并确认。应用使用全球云端点；世纪互联、政府云等独立云端点不在本次支持范围内。企业条件访问策略可能禁止设备码授权。

OneDrive 按目录逐层查找或创建，非空文件使用 5 MiB 分片上传，恢复时查询微软返回的 `nextExpectedRanges`。远端文件名追加任务 ID（保留扩展名），防止覆盖用户同名文件，并允许最后一次响应丢失后找回提交结果。已有结果只重新创建分享，不重新上传。上传会话 URL 是敏感恢复元数据，保存在本机 SQLite，不记录到日志。

分享调用 `createLink` 创建 `view/anonymous` 链接，组织可能禁止匿名分享。指定有效期时发送 `expirationDateTime`，展示服务端实际返回的有效期，不保证“永久”。分享失败保留远端文件，修改设置或组织策略后可按平台重试。

旧双盘任务的 OneDrive 项迁移为暂停，可手动继续；默认复制模板自动升级为三平台，自定义模板保持原样，可添加 `{onedrive_url}` 与 `{onedrive_code_line}`。

## 环境变量

复制 `.env.example` 为 `.env` 仅用于本机开发，不要提交 `.env`。

```text
BAIDU_APP_KEY=
BAIDU_SECRET_KEY=
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
ONEDRIVE_CLIENT_ID=
```

这些值是开发期可选初值。生产使用建议在设置页写入 Windows Credential Manager。

## OAuth 回调

百度使用设备码流程，没有本地回调地址。

Google 使用官方 installed app loopback flow。每次授权时程序先监听随机可用端口，再把精确的 `http://127.0.0.1:{port}` 作为本次 `redirect_uri` 发给 Google。回调只接受一次，校验随机 `state`，并用本次 PKCE verifier 交换 Token。不要为 Google 创建 Web application 类型的 OAuth 客户端。

## 数据库与日志

默认数据目录：

```text
%APPDATA%\com.dualdrive.shipper\
```

数据库文件是 `dual-drive-assistant.db`，日志位于同目录的 `logs` 子目录。SQLite 使用 WAL 模式。设置页提供“打开日志目录”按钮。

为兼容旧版本，数据库表中可能仍存在名称以 `quark_` 开头的历史列。这些列不会被当前版本读取、显示或更新，保留它们只是为了避免升级时执行破坏性迁移。

## 项目目录

```text
src/
  components/          Vue 拖放、三平台任务、结果、历史和设置组件
  services/            Tauri 命令调用封装
  types/               TypeScript 统一类型
  utils/               格式化与测试
src-tauri/src/
  adapters/baidu.rs    BaiduDriveAdapter
  adapters/google.rs   GoogleDriveAdapter
  adapters/onedrive.rs OneDriveAdapter
  auth.rs              百度 OAuth 与统一认证管理器
  auth/google.rs       Google loopback OAuth、PKCE 与 Token Refresh
  auth/onedrive.rs     Microsoft 设备码、服务端轮询节流与 Token Refresh
  database.rs          SQLite 任务、历史、设置与兼容迁移
  task_manager.rs      Hash、去重、三平台并发与独立控制
  share_manager.rs     三平台复制文本模板
  secret_store.rs      Windows Credential Manager
  logging.rs           JSONL 日志与脱敏
  commands.rs          Tauri 命令边界
docs/API_RESEARCH.md   官方 API 核验记录
.codex/skills/         项目维护经验 Skill
```

## 常见错误

“Google `redirect_uri_mismatch`”：确认创建的是 `Desktop app` OAuth 客户端，不是 Web application；不要手工填写固定回调路径。

“Google OAuth 应用尚未验证”：开发期把登录账号加入 OAuth consent screen 的测试用户；发布给其他用户前按 Google 要求完成应用发布或验证。

“Google 分享被拒绝”：确认已启用 Drive API，并检查 Google Workspace 管理员是否禁止 `anyone` 公开分享。文件已经上传成功时，只需稍后重试 Google 分享，不会重新上传。

“百度 Token 已失效或权限不足”：程序会先尝试 Refresh；仍失败时，在设置页清除授权后重新绑定，并确认应用已获得网盘权限。

“百度上传路径不合法”：把默认目录改为开放平台实际分配的 `/apps/应用目录/`，不要直接上传到用户网盘任意根目录。

“HTTP 429 / rateLimitExceeded / errno 31034”：平台触发限流。程序会进行有上限的指数退避，达到设置的重试上限后保留为可重试状态，不会无限请求。

“link.exe / Windows SDK 未找到”：安装 Visual Studio 2022 Build Tools 的 C++ 桌面工作负载，然后在 Developer Command Prompt 中重新构建。

“磁盘空间不足”：`src-tauri/target` 是可重建的 Cargo 编译缓存。在确认没有需要保留的构建产物后，可运行 `cargo clean --manifest-path src-tauri/Cargo.toml`。

## 隐私与安全

隐私说明见 [PRIVACY.md](PRIVACY.md)。`.gitignore` 已排除 `.env`、Token、本地数据库、日志、构建目录和本地工作数据。不要把真实账号、Client Secret、Cookie 或测试文件提交到仓库。

## 验收边界

源码、单元测试、前端生产构建和 Windows 安装包可在无账号环境下验证。真实百度与 Google 上传、账号配额、组织分享策略和 OAuth consent screen 状态必须使用用户自己的官方应用与账号完成；仓库不提供公共测试账号或凭据。
