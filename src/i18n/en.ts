// Chinese source strings are stable keys. User filenames, paths and templates are never translated.
export const en: Record<string, string> = Object.fromEntries(`
选择文件上传|Choose files to upload
选择文件|Choose files
上传整个文件夹|Upload a folder
点 + 选文件，也可以直接拖进来|Click + to choose files, or drop them here
整个文件夹也能上传，里面的子目录和空文件夹也会保留。|Upload a whole folder, including its subfolders and empty folders.
队列就绪|Queue ready
队列已暂停|Queue paused
队列处理中|Queue active
暂停后不再开始下一项，正在上传的文件会继续完成。|Pausing stops new items from starting. Current uploads will finish.
暂停队列|Pause queue
继续队列|Resume queue
文件夹|Folder
保留目录|Folder path
文件夹已创建|Folder created
该文件仍在队列中，请等待当前处理结束后重试|This item is still queued or running. Wait for it to finish before retrying.

文件上传成功，当前百度个人开放 API 文档未确认支持自动创建公开分享|Uploaded. Automatic public sharing is not confirmed for Baidu's personal open API.
默认模板会随界面语言生成文案；自己修改过的模板保持原文。|Default share text follows the interface language. Custom templates keep their original text.
选择 JSON 文件|Choose JSON file
180 天|180 days
30 天|30 days
7 天|7 days
一次拖入 · 自动上传|Drop once · Upload automatically
一次拖入，送达已启用的网盘。|Drop once. Deliver to your enabled drives.
上传中|Uploading
上传位置和其他设置（一般不用改）|Upload folders and other settings (usually no changes needed)
上传时间|Uploaded
上传目录|Upload folders
上传进度|Upload progress
不用把网盘密码填进软件，只在网盘自己的网页登录。|Sign in on the drive's own website. Never enter your drive password here.
个文件|files
也可以选中验证码，按 Ctrl+C 复制。|You can also select the code and press Ctrl+C.
从百度应用详情复制，不是网盘密码|Copy from your Baidu app details, not your drive password
从百度应用详情复制|Copy from your Baidu app details
从微软应用概述复制，不是邮箱或密码|Copy from the Microsoft app overview, not your email or password
以 .apps.googleusercontent.com 结尾|Ends with .apps.googleusercontent.com
以微软平台策略为准|Subject to Microsoft policy
任务策略|Task settings
保存中…|Saving…
保存并去登录|Save and sign in
保存设置|Save settings
保留当前账号|Keep current account
停用|Disable
停用或移除不删除网盘文件、历史记录或登录配置。已经开始的上传会继续；排队中的任务和手动重试按当前开关执行。|Disabling or removing keeps your files, history and sign-in settings. Active uploads continue; queued tasks and manual retries use the current plugin settings.
先导入 Google 配置文件，或展开手动填写。|Import your Google configuration file, or open manual entry below.
先绑定，再拖文件|Connect first, then drop files
先解除本机绑定，再登录另一个账号。网盘里的文件不会删除；旧账号的上传任务请先暂停。|Disconnect here, then sign in with another account. Your drive files will stay. Pause uploads for the old account first.
全部分享信息已复制|All share details copied
分享有效期|Link expiry
分享结果保存在本机，再次复制不需要重新上传。|Share details are saved locally. Copy them again without uploading again.
创建分享中|Creating share link
历史分享文案已复制|Share details copied from history
历史去重|Reuse uploaded files
历史记录|History
去绑定网盘|Connect a drive
双盘发货助手|Drive Delivery
发货任务|Uploads
取消|Cancel
可用变量：{filename}、{baidu_url}、{baidu_code_line}、{google_url}、{google_code_line}、{onedrive_url}、{onedrive_code_line}|Variables: {filename}, {baidu_url}, {baidu_code_line}, {google_url}, {google_code_line}, {onedrive_url}, {onedrive_code_line}
同时处理文件数|Concurrent files
启用|Enable
在 Google 网页选择账号并同意，完成后这里会自动变成“已绑定”。|Choose your account and approve on Google's website. This page will show Connected automatically.
处理中|Processing
处理失败|Failed
处理完成后自动复制可用分享结果|Automatically copy available share details when finished
处理队列|Upload queue
复制 Google|Copy Google
复制 OneDrive|Copy OneDrive
复制全部|Copy all
复制文本模板|Copy template
复制百度|Copy Baidu
复制验证码|Copy code
失败|Failed
失败自动重试|Automatic retries
失败，可重试|Failed · Retry available
如果提示“首次设置”，说明这份软件还没有该网盘的应用配置。请给你软件的人提供配置，或展开下方教程自己设置一次；网盘密码只在官方网页输入。|If you see First-time setup, this copy of the app needs a drive configuration. Ask the person who gave you the app for it, or follow the guide below once. Enter your drive password only on the official website.
安装并启用|Install and enable
完成一次上传后，文件与分享信息会出现在这里。|Your files and share details will appear here after an upload.
官方接入不可用|Official API unavailable
官方设置页| settings
官方配置里有这一项才填写|Only fill this in if included in your official configuration
实时任务|Live tasks
导入 Google 配置文件（推荐）|Import Google configuration (recommended)
导入插件|Import plugin
已停用|Disabled
已取消|Cancelled
已启用|Enabled
已复制，再复制一次|Copied · Copy again
已复制，去网页按 Ctrl+V 粘贴。|Copied. Press Ctrl+V on the website to paste.
已完成|Completed
已暂停|Paused
已生成|Ready
已经可以上传了，下次打开不用再登录。|Ready to upload. You won't need to sign in again next time.
已绑定|Connected
已绑定，下次打开无需重复登录|Connected. No need to sign in again next time.
已解除本机绑定。应用配置已保留，再点绑定即可换账号。网盘里的文件不会删除。|Disconnected on this computer. App configuration is saved; connect again to switch accounts. Your drive files stay untouched.
平台分享信息已复制|Drive share details copied
应用初始化失败|Unable to start the app
应用密钥（Client Secret，可选）|Client Secret (optional)
应用密钥（Secret Key）|Secret Key
应用编号（App Key / API Key）|App Key / API Key
应用编号（Client ID）|Client ID
应用编号（应用程序 / 客户端 ID）|Application (client) ID
应用配置不正确或尚未开通。展开“首次设置”核对，或请软件提供者帮你配置。|The app configuration is incorrect or not enabled. Check First-time setup or ask your app provider for help.
微软网盘|OneDrive
必须位于开放平台分配的 /apps/应用目录/ 下|Must be within the /apps/app-folder/ assigned by Baidu
打开|Open\u0020
打开你要用的网盘，再到下面登录。以后拖入文件，只发到已启用的网盘。|Enable the drives you want, then sign in below. Dropped files go only to enabled drives.
打开日志目录|Open log folder
把这个验证码填到刚打开的官方网页：|Enter this code on the official website that just opened:
拖入文件或文件夹|Drop files or folders here
按文件名搜索|Search by filename
按日期创建子目录，例如 2026-08-29|Create a subfolder by date, e.g. 2026-08-29
按需开启|Choose your drives
换一个账号|Switch account
授权未完成|Authorization not completed
排查信息|Troubleshooting
提取码：|Access code:\u0020
插件导入失败，请选择支持的 .drive-plugin.json 文件（不超过 4 KB）|Import failed. Choose a supported .drive-plugin.json file (up to 4 KB).
插件文件不能超过 4 KB|Plugin files must be no larger than 4 KB
插件文件是什么？|What is a plugin file?
插件文件用于安装本软件内置的官方网盘适配器，不包含运行代码。百度、Google Drive、OneDrive 已随软件提供；接入其他网盘需要更新软件适配器。|Plugin files install official drive adapters bundled with this app; they contain no executable code. Baidu, Google Drive and OneDrive are included. Other drives require an adapter update to the app.
插件设置已保存|Plugin settings saved
数据仅保存在本机|Data is stored only on this computer
数据库：|Database:\u0020
文件|File
文件格式不对，请选择 Google 下载的 JSON 文件，不要选截图或文档。|Choose the JSON file downloaded from Google, not a screenshot or document.
文件里没有有效的应用编号，请重新下载桌面应用配置。|No valid client ID was found. Download the desktop app configuration again.
断点续传|Resume uploads
无链接|No link
日志：|Logs:\u0020
暂停|Pause
暂时不绑|Not now
暂时连不上 Google。先试试浏览器能否打开 Google，再回来重试。|Can't reach Google. Check that Google opens in your browser, then try again.
暂时连不上网盘，请检查网络后重试。|Can't reach the drive. Check your connection and try again.
暂未生成分享链接|No share link yet
有效期：|Expires:\u0020
未安装|Not installed
未绑定|Not connected
本地 SQLite|Local SQLite
本地安全运行|Stored locally
本地数据与日志|Local data and logs
本次未启用|Not enabled for this task
松开后自动校验并上传已启用的网盘，支持多文件和完整文件夹|Release to verify and upload to enabled drives. Multiple files and whole folders supported.
查看各网盘支持哪些功能|See what each drive supports
查看官方说明|Official instructions
校验文件|Verifying file
正在打开官方登录页…|Opening the official sign-in page…
正在打开本地数据库并恢复任务…|Opening local data and restoring tasks…
正在读取历史记录…|Loading history…
正在读取文件…|Reading files…
永久或官方最长有效期|No expiry or the provider's maximum
永久有效|No expiry
没有文件，手动填写|No file? Enter manually
流式分片|Streamed chunks
浏览器没能打开。请检查电脑的默认浏览器，再点一次对应的打开按钮。|The browser could not open. Check your default browser, then try the open button again.
点“绑定网盘”，按提示登录并同意。绑定好了再拖文件，之后不用重复登录。|Open Connect drives, sign in and approve. Then drop your files. No need to sign in every time.
状态|Status
登录你要用的网盘，照着网页点“同意”就行。|Sign in to your drive and approve on its website.
百度|Baidu
百度当前支持上传；发货用的分享链接仍需到百度网盘里手动创建。|Baidu supports uploads. Create delivery share links manually in Baidu Drive.
百度网盘|Baidu Drive
百度网盘、Google Drive 与 OneDrive 并行上传，支持真实进度、断点续传、独立重试和历史去重。|Upload to enabled Baidu, Google Drive and OneDrive plugins with live progress, resume, individual retries and upload history.
百度默认目录|Baidu default folder
Google Drive 默认目录|Google Drive default folder
OneDrive 默认目录|OneDrive default folder
移除插件|Remove plugin
程序会在 My Drive 中按层级查找或创建目录|Folders are found or created inside My Drive
第一次用，从这里开始|New here? Start here
第一次用？先登录你要用的网盘|First time? Connect the drives you want to use
等你在网页确认…|Waiting for approval…
等待上传|Waiting to upload
等待处理|Waiting
绑定好了，去发货|Connected · Start uploading
绑定成功！回到“发货任务”，拖入文件就能上传。|Connected! Go to Uploads and drop files to start.
绑定没有完成，请重试。也可以展开下方“排查信息”，请软件提供者帮你看看。|Could not connect. Try again, or open Troubleshooting below and ask your app provider for help.
绑定网盘|Connect drives
继续|Resume
网盘插件|Drive plugins
自动创建目录；文件名追加任务 ID，避免覆盖|Creates folders automatically; adds a task ID to filenames to avoid overwriting
解除绑定|Disconnect
设置已保存|Settings saved
该平台任务已取消|Drive task cancelled
该平台任务已暂停|Drive task paused
请先填写下面的应用配置。这不是你的网盘账号或密码。|Enter the app configuration below first. This is not your drive username or password.
请检查应用类型、测试用户和公共客户端设置。更多原因可在本机日志中查看。|Check the app type, test users and public client settings. Local logs may contain more details.
请选择 Google 下载的应用配置 JSON 文件，文件应小于 64 KB。|Choose the app configuration JSON downloaded from Google (up to 64 KB).
请选择“桌面应用”下载的配置文件，网页应用或服务账号的文件不能用。|Choose a Desktop app configuration. Web app and service account files cannot be used.
请选择本软件支持的网盘插件文件|Choose a drive plugin supported by this app
谷歌网盘|Google Drive
还差首次设置：请按下面的说明准备一次，保存后就不用再填了。|One-time setup is needed. Follow the guide below; once saved, you won't need to enter it again.
还没有匹配的历史记录|No matching history
还没有启用网盘，请在上面点“安装并启用”或“启用”。|No drives enabled. Click Install and enable or Enable above.
这个文件太大了，请选择 Google 下载的应用配置 JSON 文件。|This file is too large. Choose the app configuration JSON downloaded from Google.
这是“让软件获得登录入口”的设置，不是填写你的网盘密码。通常由给你软件的人准备；已经有配置就直接填，只有第一次需要。|This gives the app a sign-in entry point; it is not your drive password. Your app provider can prepare it. If you already have the configuration, enter it here. You only need to do this once.
这次没有完成同意操作。重新点绑定，在官方网页登录并同意；公司账号若被拦截，需要管理员允许。|Approval was not completed. Connect again, sign in and approve on the official website. Work accounts may need administrator approval.
这次登录已超时。再点一次绑定，用新页面或新验证码继续。|Sign-in timed out. Connect again to get a new page or code.
这次登录已过期。重新点绑定，使用新出现的验证码或登录页面。|Sign-in expired. Connect again and use the new code or page.
选一个网盘点“绑定” → 在官方网页登录并同意 → 回到这里看到“已绑定”。绑定后会记住，下次不用再来一遍。|Click Connect for a drive → sign in and approve on its official website → return here to see Connected. We'll remember it for next time.
选择从 Google 下载的 JSON 文件，程序自动填写，不会上传这个文件。|Select the JSON file downloaded from Google. We'll fill in the settings locally without uploading it.
部分完成|Partially completed
配置已保存，直接登录就行。|Configuration saved. Just sign in.
配置已读好，直接点“保存并去登录”。|Configuration loaded. Click Save and sign in.
配置文件内容不完整，请重新下载。|The configuration file is incomplete. Download it again.
重新打开登录页|Reopen sign-in page
重试|Retry
首次使用还需要应用配置，点绑定后会显示操作说明。|First-time use needs app configuration. Click Connect for instructions.
首次设置 · 不知道怎么填就看这里|First-time setup · Start here if you're unsure
首次设置已完成 · 需要时修改配置|Setup complete · Edit configuration if needed
验证码没能自动复制，请选中下方验证码，按 Ctrl+C 复制。|The code could not be copied automatically. Select it below and press Ctrl+C.
已停止等待。网页上的登录也请关闭，需要时可重新绑定。|Stopped waiting. Close the sign-in page too. You can connect again later.
请先在网盘插件中启用至少一个网盘|Enable at least one drive in Drive plugins first
请先启用该网盘插件，再重试任务|Enable this drive plugin before retrying
插件文件格式不正确|Invalid plugin file format
此插件需要其他版本的软件，或尚未支持该网盘|This plugin needs a different app version or an unsupported drive adapter
所有平台均处理完成|All selected drives completed
部分平台已完成，其余平台可单独重试|Some drives completed. Retry the others individually.
平台处理未全部完成，可按平台重试|Some tasks did not complete. Retry each drive as needed.
已命中历史记录，仅上传缺失的平台|Found in history. Uploading only to missing drives.
已命中上传记录，仅重新创建失效的分享|Found uploaded files. Recreating expired share links only.
该文件已经上传过，已直接显示历史分享结果|Already uploaded. Showing saved share details.
`.trim().split("\n").map(line => { const i = line.indexOf("|"); return [line.slice(0, i), line.slice(i + 1)]; }));
