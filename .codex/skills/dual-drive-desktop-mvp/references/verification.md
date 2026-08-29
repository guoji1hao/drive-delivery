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
cmd.exe /d /s /c "`"$vsDevPath`" -arch=x64 -host_arch=x64 >nul && cargo test --manifest-path src-tauri\Cargo.toml"
```

NSIS 安装包：

```powershell
cmd.exe /d /s /c "`"$vsDevPath`" -arch=x64 -host_arch=x64 >nul && npm.cmd run tauri build -- --bundles nsis"
```

启动 Release 可执行文件后，确认窗口标题为“双盘发货助手”、进程响应、数据库存在且启动日志非空。没有用户自己的平台授权时，明确将真实双盘上传与分享列为未执行，不得用构建成功代替线上 API 验收。
