---
name: dual-drive-desktop-mvp
description: Maintain, debug, test, or package the 双盘发货助手 Tauri 2 + Vue 3 Windows desktop project. Use for changes to its Baidu-only drag-and-drop tasks, SQLite history, BaiduDriveAdapter, OAuth, upload/resume/share workflow, or NSIS release build.
---

# 双盘发货助手项目维护

## 开始工作

先阅读仓库根目录 `README.md` 和 `docs/API_RESEARCH.md`。涉及百度接入时，重新核验官方最新文档或官方 SDK；平台能力容易变化，不得只依赖本 Skill 的日期快照。

检查 `git status`，保留用户已有改动。项目文件使用 UTF-8，PowerShell 调用 npm 时使用 `npm.cmd`。

## 模块边界

Vue 组件位于 `src/components`，只负责交互和展示；所有系统能力通过 `src/services/api.ts` 调用 Tauri 命令。

Rust 的统一平台接口位于 `src-tauri/src/adapters/mod.rs`，当前只有 `BaiduDriveAdapter`。平台协议细节只能留在 Adapter，不要泄露到 `task_manager.rs` 或 Vue 组件。

任务状态与重试在 `task_manager.rs`，SQLite 与迁移在 `database.rs`，百度凭据和 Token 在 `auth.rs` 与 `secret_store.rs`，分享文案在 `share_manager.rs`，日志脱敏在 `logging.rs`。

不要把 Access Token、Refresh Token、Secret、Cookie 或真实账号写入数据库、日志、测试、README 或 Git。没有官方依据的能力返回 `Unavailable` 或清晰 notice，不创建模拟链接。

## 当前官方边界

百度使用官方 OAuth 2.0 设备码、Refresh Token 与三步上传。上传分片固定为官方 SDK 默认的 4 MiB，一次只保留一个分片。个人开放 API 的公开分享创建当前未确认支持，因此只能显示真实限制。

夸克支持已经从当前版本移除。不得恢复 Skill 检测、自动安装、WSL/CLI 调用、Cookie 流程或前端入口，除非用户明确要求重新接入并且新的官方桌面 API 资料已经通过实时核验。

旧 SQLite 表可能含 `quark_` 前缀列，旧任务 JSON 和设置 JSON 也可能带有已移除字段。只保留无损反序列化和数据库升级兼容；运行时、查询结果、界面和新写入数据不得继续消费这些字段。不要擅自卸载用户在应用外安装的 Skill 或 CLI。

## 修改检查

状态字段必须在 `models.rs` 与 `src/types/index.ts` 同步。新增 SQLite 字段时既修改建表语句，也添加旧数据库迁移测试。进度必须来自实际完成字节或官方 CLI 的 NDJSON，不得用计时器伪造。

重复文件先用大小和 SHA-256 查询历史；有效分享直接复用，过期分享只重新创建。旧历史中没有百度远端结果时必须正常上传百度，不能因为旧平台曾成功就误判为可直接复用。

完整验收命令见 `references/verification.md`。打包前生成或检查 Tauri 图标，NSIS 产物位于 `src-tauri/target/release/bundle/nsis`。
