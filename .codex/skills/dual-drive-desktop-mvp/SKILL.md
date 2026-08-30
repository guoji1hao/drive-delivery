---
name: dual-drive-desktop-mvp
description: Maintain, debug, test, or package the 双盘发货助手 Tauri 2 + Vue 3 Windows desktop project. Use for changes to its drag-and-drop tasks, SQLite history, BaiduDriveAdapter, QuarkDriveAdapter, OAuth, upload/resume/share workflows, or NSIS release build.
---

# 双盘发货助手项目维护

## 开始工作

先阅读仓库根目录 `README.md` 和 `docs/API_RESEARCH.md`。涉及百度或夸克接入时，重新核验官方最新文档或官方仓库；平台能力容易变化，不得只依赖本 Skill 的日期快照。

检查 `git status`，保留用户已有改动。项目文件使用 UTF-8，PowerShell 调用 npm 时使用 `npm.cmd`。

## 模块边界

Vue 组件位于 `src/components`，只负责交互和展示；所有系统能力通过 `src/services/api.ts` 调用 Tauri 命令。

Rust 的统一平台接口位于 `src-tauri/src/adapters/mod.rs`。百度和夸克平台差异只能留在各自 Adapter，不要泄露到 `task_manager.rs` 或 Vue 组件。

任务状态与双平台独立重试在 `task_manager.rs`，SQLite 与迁移在 `database.rs`，百度凭据和 Token 在 `auth.rs` 与 `secret_store.rs`，分享文案在 `share_manager.rs`，日志脱敏在 `logging.rs`。

不要把 Access Token、Refresh Token、Secret、Cookie 或真实账号写入数据库、日志、测试、README 或 Git。没有官方依据的能力返回 `Unavailable` 或清晰 notice，不创建模拟链接。

## 当前官方边界

百度使用官方 OAuth 2.0 设备码、Refresh Token 与三步上传。上传分片固定为官方 SDK 默认的 4 MiB，一次只保留一个分片。个人开放 API 的公开分享创建当前未确认支持，因此只能显示真实限制。

夸克使用官方 Skill/组件，Windows 通过 WSL 调用。不要把 `quark-drive.cjs` 路径暴露到设置、环境变量或前端；绑定只能调用一个后端入口，由该入口实时检测官方全局命令、常见 WSL Skill 目录与 `~/.quarkclouddrive`。缺失时自动运行官方安装脚本，存在时不得重复安装，然后继续授权。

截至 2026-08-30，夸克未公开面向普通 Windows 桌面应用的 Client ID 注册和完整 REST API 契约。不要为了模仿百度直连而猜测端点、字段或 Cookie 流程；未来获得官方文档后只替换 Adapter。

## 修改检查

状态字段必须在 `models.rs` 与 `src/types/index.ts` 同步。新增 SQLite 字段时既修改建表语句，也添加旧数据库迁移测试。进度必须来自实际完成字节或官方 CLI 的 NDJSON，不得用计时器伪造。

平台失败不得覆盖另一个平台的成功结果。重试指定平台时不得重复上传已成功的平台。重复文件先用大小和 SHA-256 查询历史；有效分享直接复用，过期分享只重新创建。

完整验收命令见 `references/verification.md`。打包前生成或检查 Tauri 图标，NSIS 产物位于 `src-tauri/target/release/bundle/nsis`。
