# 双盘发货助手隐私说明

更新日期：2026-10-21。

双盘发货助手是本地优先的桌面工具。用户主动拖入的文件只会按用户授权的设置上传到百度网盘、Google Drive 和 Microsoft OneDrive，不会被发送到本项目维护者的服务器。

程序在本地保存文件路径、文件名、文件大小、SHA-256 Hash、任务状态、远端路径和分享结果，用于断点恢复、历史搜索和重复文件识别。默认位置是 `%APPDATA%\com.dualdrive.shipper\dual-drive-assistant.db`。

百度、Google 与 Microsoft 的 Access Token、Refresh Token 和 OAuth 应用凭据通过 Windows Credential Manager 保存，不写入 SQLite、日志或 Git 仓库。Google 与 OneDrive upload session 只作为断点恢复元数据保存在本地 SQLite 中，不写入日志。

本地日志只记录必要的任务与错误信息，并对 Token、Secret、Cookie 和 Authorization 等敏感字段脱敏。用户可以在设置页打开日志目录并自行删除日志。

卸载程序不会自动删除用户的网盘文件。若需撤销授权，请先在应用设置或对应网盘官方账号管理页面解除授权，再按需删除本地应用数据目录。

本项目不收集广告标识，不出售个人信息，也不内置第三方统计 SDK。平台自身对文件与账号数据的处理分别受百度网盘、Google 和 Microsoft 的隐私政策与服务协议约束。


## 浏览器扩展 / Browser extension

浏览器版无需本地服务或桌面助手，文件直接传给用户选定的网盘。OAuth 令牌与百度个人应用密钥仅存于 chrome.storage.session，浏览器结束会话后清除，不写入同步存储。配置中的公开 Client ID、语言、上传位置和最近 200 条完成结果保存在扩展本地存储；历史不保存文件内容。未完成队列、File 对象与续传地址仅保留在上传页面内存。

identity 用于官方网盘登录，storage 用于上述本地/会话数据，host_permissions 仅用于列明的官方网盘 API、登录和上传域名。不注入网页脚本，不收集浏览记录，不使用统计 SDK，不向维护者传输文件或凭据。公开分享默认关闭；用户开启后，Google/OneDrive 持链接者可能访问文件。清空历史不会删除网盘文件；断开绑定不会撤销平台端已有授权。

The extension sends files directly to selected providers. Tokens and personal Baidu secrets use session storage only; settings and up to 200 completed result records use local storage. Pending files/upload sessions remain in tab memory. There is no telemetry, content script, browsing-history collection or maintainer relay. Public sharing is opt-in. Clearing history does not delete cloud files; revoke provider access through the provider account settings.
