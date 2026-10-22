# 浏览器版安装 / Browser extension

这是独立浏览器扩展，不需要安装桌面助手。不需要 Node.js 或 Rust 即可使用已打包的扩展。

解压下载的 `drive-delivery-browser-0.1.0.zip` 到一个固定文件夹。Chrome 地址栏输入 `chrome://extensions`，打开“开发者模式”，点“加载已解压的扩展程序”，选择包含 `manifest.json` 的文件夹。Edge 对应地址是 `edge://extensions`。之后点工具栏的扩展图标即可打开上传页。

Unzip the browser release to a permanent folder. Open `chrome://extensions` (Chrome) or `edge://extensions` (Edge), enable Developer mode, choose Load unpacked, and select the folder containing manifest.json. Click the toolbar icon to open the uploader.

## 绑定网盘 / Connect drives

发布者尚未提供公共 OAuth 应用配置。打开网盘卡片中的“首次设置”，照着官方入口配置自己的应用。Google 使用 Chrome Extension OAuth 客户端，不能使用桌面版的 Desktop 客户端；页面显示扩展 ID，并可以下载已填入 Client ID 的 manifest.json，替换扩展目录同名文件后刷新扩展即可。Google 原生授权仅支持 Chrome；Edge 可使用 OneDrive 与百度。

OneDrive 注册应用的回调平台必须选 Single-page application (SPA)，回调地址必须与扩展页面显示的地址完全一致；使用授权码 + S256 PKCE，不需要 Client Secret。百度需使用自己的 App Key、Secret Key 和开放平台分配的 `/apps/应用目录/`。不要把自己的 Secret Key 写入发布包或公开仓库。

The release does not include a public OAuth app registration. Each drive's setup panel links to its official console. Google requires a Chrome Extension OAuth client and Chrome (not Edge); a Desktop client will not work. Download the generated manifest, replace the extension's manifest.json and reload. OneDrive needs an SPA registration with the exact displayed redirect URI and uses PKCE without a secret. Baidu requires your own app credentials and assigned app folder. Never distribute a shared Baidu secret.

## 上传 / Upload

绑定并勾选网盘后，点“+”选择文件或点“上传整个文件夹”。文件夹选择使用浏览器 File System Access API 保留空目录；不支持此 API 时会提示兼容选择器无法包含空目录。百度不支持零字节文件，其他文件继续处理。百度和 OneDrive 文件名附加稳定任务编号避免覆盖。

队列暂停只停止后续项目，已开始的项目继续完成。失败重试仅针对失败的平台，保留本页上传会话；分享失败不重新上传。公开分享默认关闭，开启会让持链接者可以查看 Google/OneDrive 文件，组织策略可能禁止分享。百度公开分享 API 未确认支持。

Connect and select your drives, then use + or Upload a folder. Native folder selection preserves empty folders; the compatibility picker cannot. Baidu does not accept zero-byte files. Baidu/OneDrive names include stable task IDs to avoid overwriting existing files. Pause stops new jobs; in-flight uploads finish. Retry preserves upload sessions in this tab. Public links are opt-in and may be blocked by organization policy.

上传期间保留此页面。关闭或刷新会丢失未完成队列与文件选择，重新选文件可能创建另一份文件；已完成结果在本地历史中。当前版本不提供浏览器重启后的断点恢复。令牌、百度 Secret Key 只保存在扩展的会话存储，关闭浏览器后清除。断开本地绑定不等于撤销云端授权；撤销请到对应网盘的官方授权管理页。

Keep the uploader tab open. Closing or refreshing loses unfinished jobs and file selections; selecting them again may create another copy. Completed results remain in local history. Cross-restart resume is not supported. Tokens and Baidu secrets stay in extension session storage and are cleared when the browser closes. Disconnect clears local credentials; revoke access through the provider's account settings.

## 源码构建 / Build

From the repository root: `npm ci`, `npm run extension:test`, `npm run extension:build`. Load `dist-extension`. Publishers may set `EXTENSION_GOOGLE_CLIENT_ID` before building; it is a public client identifier, not a secret. The manifest's public key stabilizes the unpacked extension ID; no private key is distributed. Store distribution may use a different assigned ID and requires its own registered OAuth client and review.

The package is not listed in Chrome Web Store or Microsoft Edge Add-ons. Automated protocol fixtures are not proof of real OAuth/provider acceptance. See repository verification notes for the exact tested boundaries.
