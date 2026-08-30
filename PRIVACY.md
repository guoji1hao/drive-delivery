# 双盘发货助手隐私说明

更新日期：2026-08-30。

双盘发货助手是本地优先的桌面工具。用户主动拖入的文件只会按用户授权的设置上传到百度网盘，不会被发送到本项目维护者的服务器。

程序在本地保存文件路径、文件名、文件大小、SHA-256 Hash、任务状态、远端路径和分享结果，用于断点恢复、历史搜索和重复文件识别。默认位置是 `%APPDATA%\com.dualdrive.shipper\dual-drive-assistant.db`。

百度 Access Token、Refresh Token、App Key 和 Secret Key 通过 Windows Credential Manager 保存，不写入 SQLite、日志或 Git 仓库。

本地日志只记录必要的任务与错误信息，并对 Token、Secret、Cookie 和 Authorization 等敏感字段脱敏。用户可以在设置页打开日志目录并自行删除日志。

卸载程序不会自动删除用户的网盘文件。若需撤销授权，请先在应用设置或对应网盘官方账号管理页面解除授权，再按需删除本地应用数据目录。

本项目不收集广告标识，不出售个人信息，也不内置第三方统计 SDK。平台自身对文件与账号数据的处理受百度网盘隐私政策与服务协议约束。
