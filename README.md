# 双盘发货助手

双盘发货助手是一款 Windows 桌面工具。把文件、多个文件或文件夹拖入窗口后，程序会为每个文件并行执行百度网盘与夸克网盘上传，展示两个平台的真实进度，按平台独立处理失败与重试，并把可用分享结果整理成可直接发给客户的文案。

本仓库是可编译运行的 MVP。它不会伪造分享链接，也不会调用 Cookie、抓包或逆向私有接口。平台能力边界基于 2026-08-30 核验到的官方资料，详见 [API 调研记录](docs/API_RESEARCH.md)。

## 技术栈

桌面壳使用 Tauri 2，界面使用 Vue 3、TypeScript 与 Vite。Rust 后端负责流式 Hash、上传、并发、断点状态、SQLite、Windows 凭据管理器、日志、剪贴板和系统目录。数据库使用 rusqlite 内置 SQLite，敏感凭据使用 keyring 写入 Windows Credential Manager。

## 当前已实现

- 单文件、多文件和文件夹拖放；文件夹会递归展开为独立文件任务。
- SHA-256 流式 Hash 与文件大小去重，不把完整文件读入内存。
- 百度与夸克任务并行，两个平台状态、重试、暂停和取消互不影响。
- 百度官方 OAuth 2.0 设备码授权、Token 自动 Refresh、安全保存与重新授权。
- 百度官方三步上传：precreate、4 MiB 分片上传、create；支持秒传判断、缺失分片续传和真实字节进度。
- 夸克官方组件一键安装、自动检测、浏览器 OAuth、上传、秒传、NDJSON 真实进度、任务恢复和自动分享；用户无需填写内部脚本路径。
- SQLite 任务和历史记录持久化；异常退出后的处理中任务会恢复为可继续状态。
- 已上传文件复用有效历史分享；分享过期时只尝试重新分享，不重复上传已有远端文件。
- 历史记录搜索、复制百度、复制夸克、复制全部、自定义文案模板与完成后自动复制。
- 429、5xx、超时与网络连接错误的有限指数退避；没有无限重试。
- JSONL 本地日志与敏感字段脱敏；界面可直接打开日志目录。
- NSIS Windows 安装包配置与应用图标。

## 已确认的限制

百度个人开放 API 当前未确认提供“创建公开分享链接”接口。百度上传成功后，界面会如实显示“文件上传成功，当前官方接口暂不支持自动分享”，不会构造假链接。百度开放平台官网存在企业分享方案，但本 MVP 不把企业能力冒充为个人开放接口。

夸克当前公开的官方接入形态是官方 Skill/组件。官方文档标明 Windows 原生不支持、Windows WSL 支持，因此本项目在 Windows 端通过 `wsl.exe` 调用官方能力。程序会自动检测官方命令及兼容 Skill 安装，不把内部文件路径暴露给用户；未安装时可在设置页点击“一键安装并绑定夸克”。

截至 2026-08-30，官方公开资料未提供面向普通 Windows 桌面应用的 Client ID 申请入口、OAuth 端点和完整 REST API 契约，因此不能诚实地实现与百度完全相同的直连 API 模式。夸克调用仍封装在独立 Adapter 中，后续官方开放桌面应用 API 后可以直接替换，不影响任务与界面模块。

平台账号、配额、会员等级和服务端策略会影响上传大小、秒传与分享结果。官方资料没有给出统一、稳定的单文件上限或通用限流阈值，因此项目不写死未经确认的数字，以服务端真实错误为准。

## 本地开发

环境要求：Windows 10/11、WebView2 Runtime、Node.js 20 或更高版本、Rust stable，以及 Visual Studio 2022 Build Tools 的“使用 C++ 的桌面开发”工作负载。

```powershell
npm.cmd install
npm.cmd run tauri dev
```

仅启动前端页面：

```powershell
npm.cmd run dev
```

运行全部现有测试和前端生产构建：

```powershell
npm.cmd test
npm.cmd run build
cargo test --manifest-path src-tauri/Cargo.toml
```

如果普通 PowerShell 尚未加载 MSVC 环境，可使用 Visual Studio Developer Command Prompt，或先运行 `VsDevCmd.bat -arch=x64 -host_arch=x64`。

## Windows 编译与安装包

```powershell
npm.cmd run tauri build -- --bundles nsis
```

构建成功后，安装包位于 `src-tauri/target/release/bundle/nsis/`，独立可执行文件位于 `src-tauri/target/release/`。不要把 `src-tauri/target` 提交到 Git。

## 百度开放平台接入

在[百度网盘开放平台](https://yun.baidu.com/open/platform)创建应用，按官方要求完成应用资料、隐私政策和能力申请，取得 App Key 与 Secret Key。首次运行后进入“设置”，填写这两个值并点击“绑定百度网盘”。应用会打开官方设备码授权页；授权成功后 Access Token、Refresh Token、App Key 和 Secret Key 均保存到 Windows 凭据管理器。

设备码流程不依赖桌面应用自行监听 OAuth 回调端口。若改为授权码模式，应按[百度 OAuth 接入指南](https://openauth.baidu.com/doc/doc.html)在开放平台登记完全一致的回调地址。本项目当前申请的 Scope 为官方 SDK 使用的 `basic,netdisk`。

百度默认目录必须位于平台分配给应用的 `/apps/应用目录/` 下。初始值 `/apps/双盘发货助手/自动发货/` 只是项目默认值；如果平台实际分配的应用目录名不同，必须在设置页改成真实目录。

## 夸克开放能力接入

夸克接入只使用[夸克网盘官方 Skill](https://github.com/quark-clouddrive/quarkclouddrive_offical)。Windows 首次需要启用 WSL，之后直接进入应用设置页点击“一键安装并绑定夸克”。程序会在默认 WSL 中检查并按需安装 Node.js、curl 和 unzip，从官方仓库下载安装脚本，然后打开官方浏览器 OAuth；不再要求用户查找或填写 `quark-drive.cjs` 路径。

如果你希望手动安装，仍可按官方仓库当前说明执行：

```bash
npx skills add https://github.com/quark-clouddrive/quarkclouddrive_offical/skills --skill quarkclouddrive
```

随后在安装后的 `quarkclouddrive` Skill 目录运行官方 `install.sh`。程序会自动检测官方全局命令、常见 Skill 目录和 `~/.quarkclouddrive` 安装目录：

```bash
bash install.sh
node scripts/quark-drive.cjs --help
```

官方 CLI 自己负责夸克 OAuth 登录状态和刷新。本应用不读取、不保存夸克 Cookie，不把夸克 Token 写入项目数据库。

## 环境变量

复制 `.env.example` 为 `.env` 仅用于本机开发。不要提交 `.env`。

`BAIDU_APP_KEY` 与 `BAIDU_SECRET_KEY` 是开发期可选初值，生产使用建议在设置页写入 Windows 凭据管理器。夸克无需环境变量或路径配置。

## 数据库与日志

默认数据目录为：

```text
%APPDATA%\com.dualdrive.shipper\
```

数据库文件是 `dual-drive-assistant.db`，日志位于同目录的 `logs` 子目录。SQLite 使用 WAL 模式。日志只记录任务 ID、错误类别和简要信息，并会遮盖 Access Token、Refresh Token、Secret、Cookie 和 Authorization 等敏感内容。

## 项目目录

```text
src/
  components/        Vue 拖放、任务、结果、历史、设置组件
  services/          Tauri 命令调用封装
  types/             前后端共享结构的 TypeScript 映射
  utils/             格式化与测试
src-tauri/src/
  adapters/          BaiduDriveAdapter、QuarkDriveAdapter 与统一接口
  auth.rs            百度 OAuth 与 Token Refresh
  database.rs        SQLite 任务、历史、设置与迁移
  task_manager.rs    Hash、去重、并发、暂停、取消、恢复与独立重试
  share_manager.rs   分享过期判断与复制模板
  secret_store.rs    Windows Credential Manager
  logging.rs         JSONL 日志与脱敏
  commands.rs        Tauri 命令边界
docs/API_RESEARCH.md 官方 API 核验记录
.codex/skills/       本项目维护经验 Skill
```

## 常见错误

“百度 Token 已失效或权限不足”：程序会先尝试 Refresh；Refresh 仍失败时，在设置页清除授权后重新绑定，并确认应用已获得网盘权限。

“百度上传路径不合法”：把默认目录改为开放平台实际分配的 `/apps/应用目录/`，不要直接上传到用户网盘任意根目录。

“未检测到夸克官方组件”：先确认 Windows 已启用 WSL，再在设置页点击“一键安装并绑定夸克”。程序会自动安装和检测官方组件；若安装失败，界面只显示安装输出中的最后一条关键错误，不会记录 Cookie 或 Token。

“夸克授权窗口没有完成”：官方 `login` 会等待浏览器 OAuth。按官方页面完成授权；超时后由用户再次点击授权，程序不会无限循环重试。

“HTTP 429 / errno 31034”：已触发平台限流。程序会按有限指数退避重试；达到设置的重试上限后保留为可重试状态。

“link.exe / Windows SDK 未找到”：安装 Visual Studio 2022 Build Tools 的 C++ 桌面工作负载，然后在 Developer Command Prompt 中重新构建。

## 隐私与安全

隐私说明见 [PRIVACY.md](PRIVACY.md)。仓库的 `.gitignore` 已排除 `.env`、Token、本地数据库、日志、构建目录和本地工作数据。请勿把真实账号、Client Secret、Cookie 或测试文件提交到仓库。

## 验收边界

源码、单元测试、前端生产构建和 Windows 安装包可以在无账号环境下验证。真实百度与夸克端到端上传、分享和配额行为必须使用你自己已审核通过的开放平台应用与已授权账号进行，不应在仓库中提供公共测试账号或凭据。
