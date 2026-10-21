---
name: dual-drive-desktop-mvp
description: Maintain, debug, test, or package the 双盘发货助手 Tauri 2 + Vue 3 Windows desktop project. Use for its Baidu and Google Drive drag-and-drop tasks, SQLite history, official OAuth adapters, resumable upload/share workflow, or NSIS release build.
---

# 双盘发货助手项目维护

## 开始工作

先阅读仓库根目录 `README.md` 和 `docs/API_RESEARCH.md`。涉及百度或 Google 接入时，重新核验官方最新文档或官方 SDK；平台能力容易变化，不得只依赖本 Skill 的日期快照。

检查 `git status`，保留用户已有改动。项目文件使用 UTF-8，PowerShell 调用 npm 时使用 `npm.cmd`。

## 模块边界

Vue 组件位于 `src/components`，只负责交互和展示；所有系统能力通过 `src/services/api.ts` 调用 Tauri 命令。

Rust 的统一平台接口位于 `src-tauri/src/adapters/mod.rs`，当前实现 `BaiduDriveAdapter`、`GoogleDriveAdapter` 与 `OneDriveAdapter`。平台协议细节只能留在各自 Adapter，不要泄露到 `task_manager.rs` 或 Vue 组件。

任务状态与独立重试在 `task_manager.rs`，SQLite 与迁移在 `database.rs`，百度认证在 `auth.rs`，Google loopback OAuth 在 `auth/google.rs`，系统安全存储在 `secret_store.rs`，分享文案在 `share_manager.rs`，日志脱敏在 `logging.rs`。

不要把 Access Token、Refresh Token、Secret、Cookie 或真实账号写入数据库、日志、测试、README 或 Git。没有官方依据的能力返回 `Unavailable` 或清晰 notice，不创建模拟链接。

## 当前官方边界

百度使用官方 OAuth 2.0 设备码、Refresh Token 与三步上传。上传分片固定为官方 SDK 默认的 4 MiB，一次只保留一个分片。个人开放 API 的公开分享创建当前未确认支持，因此只能显示真实限制。

Google 使用 Desktop app OAuth、系统浏览器、随机 `127.0.0.1` 回调、S256 PKCE 和 state。只申请 `drive.file`。非空文件使用 resumable session 与 8 MiB 分片；恢复必须先用空 PUT 查询服务器 `308 Range`，未知状态查询失败时不得贸然创建新 session，以免重复文件。session URI 可作为恢复元数据存 SQLite，但不得写日志。

Google 分享使用 `permissions.create` 的 `anyone/reader` 和 `files.webViewLink`。`anyone` 权限没有提取码，且不能设置 `expirationTime`；Workspace 组织策略拒绝公开分享时保留上传结果，只重试分享。

夸克支持已经从当前版本移除。不得恢复 Skill 检测、自动安装、WSL/CLI 调用、Cookie 流程或前端入口，除非用户明确要求重新接入并且新的官方桌面 API 资料已经通过实时核验。

旧 SQLite 表可能含 `quark_` 前缀列，旧任务 JSON 和设置 JSON 也可能带有已移除字段。只保留无损反序列化和数据库升级兼容；运行时、查询结果、界面和新写入数据不得继续消费这些字段。不要擅自卸载用户在应用外安装的 Skill 或 CLI。

## 修改检查

状态字段必须在 `models.rs` 与 `src/types/index.ts` 同步。新增 SQLite 字段时既修改建表语句，也添加旧数据库迁移测试。进度必须来自实际完成字节或官方 CLI 的 NDJSON，不得用计时器伪造。

重复文件先用大小和 SHA-256 查询历史；有效分享直接复用，过期分享只重新创建。任一平台缺少远端结果时只补传该平台，成功平台不得被重新上传。旧版百度单盘任务反序列化时给 Google 补成暂停状态，允许用户单独继续。

E 盘空间有限时不要同时保留 debug 和 release 两套大型 Cargo 缓存。只清理可重建的 `src-tauri/target`，使用 `cargo test --release` 后复用同一 release 依赖构建 NSIS。

完整验收命令见 `references/verification.md`。打包前生成或检查 Tauri 图标，NSIS 产物位于 `src-tauri/target/release/bundle/nsis`。

## OneDrive 接入经验（2026-10-21）

微软授权独立放在 `auth/onedrive.rs`，使用全球云 common 租户设备码。Client ID 必须是 UUID；只请求 Files.ReadWrite 和 offline_access。设备码保留在后端，以随机句柄暴露给前端，服务端保存轮询间隔并处理 slow_down；刷新令牌串行化，清除授权也清除挂起请求。企业条件访问可能拒绝设备码流，不能声称所有组织账号可用。

OneDrive 分片固定 5 MiB，为 320 KiB 整数倍。uploadUrl 是预授权 URL，不携带 Graph Bearer，不允许重定向，只允许微软 HTTPS 上传域名。错误不输出 URL；GET 查询失败保留会话，仅 404/410 且结果不存在才重新创建。续传记录绑定 drive ID，账号变化时拒绝继续。文件名追加稳定任务 ID，完成响应丢失时通过同路径与大小找回；不能改回覆盖模式。

新增平台须同步 Rust/TS 模型、建表/旧表列迁移、任务 JSON 默认值与暂停迁移、去重/状态聚合、设置默认模板迁移、命令注册、历史查询及复制入口。旧自定义模板不覆盖。分享失败重试应根据远端结果决定只分享，不能依赖被 retry 重置的状态。异常退出恢复应检查每个平台，不能只检查总状态，因为总状态可能已是 partial_success。

页面检查使用 `scripts/ui-smoke.js`，通过 Playwright CLI `run-code --filename scripts/ui-smoke.js` 执行；需先启动 127.0.0.1:1420。它只 Mock Tauri IPC，不访问网盘。输出图片在 output/playwright，测试目录忽略 Git。没有真实凭据时如实报告本地验收边界。

AOCI 已初始化项目级配置和索引骨架；未加载本项目 MCP 的会话不得 scan 或声称索引 aligned，应在重新打开项目后按实时 Guide 建立索引。

## 面向普通用户的绑定引导（2026-10-21）

绑定交互统一在 DriveBindingCard.vue，平台中文教程与安全导入解析位于 utils/driveBinding.ts。SettingsView 只组合绑定卡片和其他设置。默认显示中文网盘名、绑定按钮和下一步，不默认铺开 Client ID、Secret、PKCE 等术语。首次缺配置时展开完整官方操作路径，不伪装成可直接登录；发布者应用注册、审核不能由前端文案代替。

AuthStatus.available 表示已保存应用配置。直接绑定必须复用此配置，不重复调用 save credentials，尤其不能用空输入覆盖保存的 Secret。仅“保存并去登录”主动写入新配置；Google 官方 Desktop JSON 可导入，限制 64 KiB 并拒绝 Web/service account 类型，不回显解析原文。

设备码复制失败不应中断授权，应保留可选中文本；浏览器打开失败保留 challenge，允许重新打开。轮询提供停止等待与超时提示；停止等待不等于撤销官方授权。SettingsView 使用 v-show 保持在切换页面时继续轮询；销毁时取消计时器并拦截迟到的前端结果。

scripts/binding-smoke.js 覆盖缺配置引导、错误类型导入、正常导入、保存后重绑不重复写配置、网络失败重试、设备码、停止/重启、切页期间完成绑定。仅 Mock IPC，不是正式账号验收。


## 网盘插件与英文支持（2026-10-21，0.3.0）

插件注册与生命周期在 src-tauri/src/plugins.rs，通过 Arc<dyn DriveAdapter> 选择内置适配器；plugins/ 中为声明式包，不执行外部代码。校验 4 KiB 上限、deny_unknown_fields、平台白名单与精确 API/版本。状态独立存 SQLite metadata，更新持锁并先落盘再更换内存。不要把此能力描述为可加载任意网盘 DLL。

任务创建时按启用插件标记未选择平台为 skipped；真正开始平台操作前再检查开关，手动重试也检查。已开始的上传可继续，暂停和取消不能被插件开关阻挡。状态聚合排除 skipped，全跳过为 cancelled；历史已成功结果继续保留。不要根据界面隐藏推断后端已停止调度。

中文源文案为翻译键，src/i18n/en.ts 与 guides.ts 分开管理界面和教程。t 在渲染时读取响应式 locale，动态文案用 tf 插值。localStorage 记住界面选择，save_ui_language 同步后端自动复制语言。默认分享文案按当前语言且仅含已有结果；用户自定义模板、文件名和路径不得翻译。安装程序同时启用 English/SimpChinese 和语言选择器。

新增语言要检查长标签、顶部导航、原生 file input 的系统语言和日期格式。自定义文件选择按钮避免原生控件文案混用语言，保留可访问的文件 input。scripts/plugins-language-smoke.js 覆盖重载记忆、安装/停用/移除/导入、拒绝未知版本、教程和用户内容不被翻译；与旧 ui-smoke、binding-smoke 一起验收。截图前滚动到页顶，避免 sticky header 出现在长图中部。真实授权、上传与原生安装仍需单独验收。
