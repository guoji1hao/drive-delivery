# 验收命令

前端：

```powershell
npm.cmd test
npm.cmd run build
```

Rust 测试需要 MSVC 环境：

```powershell
$vswherePath = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$vsInstallPath = & $vswherePath -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$vsDevPath = Join-Path $vsInstallPath 'Common7\Tools\VsDevCmd.bat'
cmd.exe /d /s /c "`"$vsDevPath`" -arch=x64 -host_arch=x64 >nul && cargo test --release --manifest-path src-tauri\Cargo.toml"
```

NSIS 安装包：

```powershell
cmd.exe /d /s /c "`"$vsDevPath`" -arch=x64 -host_arch=x64 >nul && npm.cmd run tauri build -- --bundles nsis"
```

启动 Release 可执行文件后，确认窗口标题为“双盘发货助手”、进程响应、数据库存在且启动日志非空。没有用户自己的百度和 Google OAuth 凭据时，明确将真实上传、断点与分享列为未执行，不得用构建成功代替线上 API 验收。

平台移除回归还要确认 Vue、Tauri 命令、TaskManager、Adapter 模块和构建依赖中不存在已移除平台的可执行入口。旧数据库兼容列可以保留，但查询模型与新任务 JSON 不得继续返回这些字段。

Google 回归至少覆盖 `drive.file` Scope、PKCE challenge、分片是 256 KiB 整数倍、`308 Range` 解析、旧任务缺失 Google 字段的迁移，以及百度成功/Google 失败时的 partial success。

## 0.4.0 验证记录（2026-10-22）

cargo test --release：28 项通过，包含目录展开、队列暂停/顺序/并发以及目录元数据数据库往返。npm test：7 项通过。前端生产构建与 NSIS 打包通过。

ui-smoke、binding-smoke、plugins-language-smoke、upload-queue-smoke 四组 Playwright 页面检查通过，均模拟 Tauri IPC；未执行真实网盘账户目录上传及原生系统对话框验收。新脚本位于 scripts/upload-queue-smoke.js。

npm 新增插件时可能升级 @tauri-apps/api，Tauri 打包要求 Rust/JS 主次版本一致。本次 API 保持 ~2.11.1，dialog 双端使用 2.7.x。Windows 编译后短暂文件占用可能导致 bundle type patch 失败，确认编译完成后单独运行 npm.cmd run tauri bundle -- --bundles nsis 重打包；本次第二次打包无此警告。

AOCI MCP 已真实加载并确认交付，但当前仓库仍为零 Entry 的初始骨架，Guide 报 baseline_missing/scope_change_required；未声称索引 aligned，本次验证依据实际源码和构建测试。
