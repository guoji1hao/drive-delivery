# 三平台适配验收（2026-10-21）

本次保留并回归 Google Drive 现有适配，新增 Microsoft OneDrive 授权、分片上传、断点恢复、分享、设置、历史和复制入口。新平台代码位于独立 auth/onedrive.rs 与 adapters/onedrive.rs。

前端 `npm.cmd test`：2 项通过。`npm.cmd run build`：类型检查及 Vite 生产构建通过。

Rust `cargo test --release --manifest-path src-tauri/Cargo.toml`：22 项通过，覆盖 Google 分片/断点、OneDrive 分片对齐与上传 URL 校验、三平台状态/去重、旧表迁移、旧任务暂停迁移、历史字段持久化、默认文案和日志脱敏。

Playwright CLI 执行 `scripts/ui-smoke.js`：三个平台任务展示、OneDrive 任务复制、设置目录保存、历史复制通过。截图位于 output/playwright/three-drive-tasks.png、three-drive-settings.png、three-drive-history.png。页面使用模拟 Tauri IPC，不能替代原生 IPC 或真实网盘端到端验收。

Windows NSIS 构建命令为 `npm.cmd run tauri build -- --bundles nsis`，安装包位于 src-tauri/target/release/bundle/nsis/双盘发货助手_0.2.0_x64-setup.exe。

真实账号 OAuth、上传、断网恢复、组织分享策略与安装运行未执行；需要用户自己的 Google Desktop OAuth 和 Microsoft 公共客户端应用。OneDrive 仅使用全球云端点；文件名追加任务 ID，以避免覆盖并恢复丢失的提交响应。

项目 AOCI 配置已生成并核对稳定二进制与本仓库路径，当前会话未加载 AOCI MCP，因此未 scan、未建立语义索引、未声称 aligned。需重新打开项目后按实时 Guide 完成索引。

## 中文绑定流程改进

前端测试现为 5 项通过，新增 Google Desktop JSON 导入及错误文案测试。Playwright scripts/binding-smoke.js 覆盖首次配置提示、错误 JSON 类型、正常导入、保存后重绑、网络失败后重试、验证码显示、停止并重新绑定、切页期间完成绑定；scripts/ui-smoke.js 保持目录保存及三平台复制回归通过。页面证据为 output/playwright/binding-simple.png、binding-google-guide.png、binding-device-code.png 和 binding-success.png。

本次不修改 Rust 协议，前端生产构建与 Windows 安装包重新构建。真实官方账号授权仍未执行；模拟界面通过不表示平台应用已完成发布审核，也不表示安装包已内置公共登录配置。
