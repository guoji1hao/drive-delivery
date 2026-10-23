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


## 0.3.0 网盘插件与英文支持（2026-10-21）

新增 plugins.rs 声明式包校验、生命周期持久化与统一适配器注册表。后端测试覆盖未知驱动/协议版本/可执行字段拒绝、大小上限、启用/移除/重新安装/数据库重载；任务测试覆盖未启用平台不阻碍单盘完成、全跳过不误报成功以及历史结果保留。默认分享文案测试覆盖英文输出、无结果平台省略与自定义模板保留。

本轮 Rust release 单元测试 26 项通过，前端 Vitest 7 项通过，Vue TypeScript 与 Vite 构建通过。scripts/ui-smoke.js、binding-smoke.js 和 plugins-language-smoke.js 均通过。新增浏览器覆盖英文/中文切换和重载记忆、插件生命周期、错误版本导入、英文教程、历史标签、复制 IPC 语言及 960 px 最小窗口导航不溢出。图片位于 output/playwright/english-tasks.png、english-plugins-guide.png、chinese-plugins.png。

浏览器测试使用 Mock Tauri IPC；插件状态落盘由真实临时 SQLite 单元测试验证。本轮没有使用真实平台账号，不代表真实 OAuth、传输、系统剪贴板或原生安装流程已经端到端验收。英文安装器配置由 NSIS 构建校验，未交互安装。平台原始错误诊断保留其原文。

AOCI 项目配置继续保留；当前会话没有加载 AOCI MCP 工具，因此未进行语义索引维护或宣称 aligned。需要重新打开项目会话后按实时 Guide 处理。

最终 NSIS 构建成功：双盘发货助手_0.3.0_x64-setup.exe，4854320 字节。SHA-256：373644373FD9D9C176358F8719CC2FF0005EE0D6AABBB4DAFE2542DDD9D9C53B。安装包包含最终前端构建 index-CXLmmHSH.js。

## 2026-10-23 UI 换肤

桌面改为固定侧栏、紧凑上传区、统一蓝灰配色；浏览器版同步面板、按钮与反馈。无新增运行依赖，无上传或授权协议修改。

npm test 7 项、extension:test 11 项通过；前端与扩展生产构建通过。ui-smoke、binding-smoke、plugins-language-smoke、upload-queue-smoke 四组 Mock IPC 回归通过。browser-skin-smoke 检查 Mock Chrome storage、390px 无横向溢出、键盘焦点与 reduced-motion。各脚本使用独立 CLI session，防止 init script 累积污染。

AOCI MCP rc17 已加载并确认交付，但现有索引为零 Entry 骨架；实时 Guide 返回 baseline_missing / scope_change_required，未声明 aligned。本次没有重建整个仓库索引。真实网盘操作、原生扩展运行和安装后交互未执行。

设计参考：https://linear.app/now/behind-the-latest-design-refresh 与 https://fluent2.microsoft.design/layout 。
