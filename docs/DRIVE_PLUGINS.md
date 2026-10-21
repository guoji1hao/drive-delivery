# 网盘插件 / Drive plugins

## 用户操作

进入“绑定网盘 → 网盘插件”，保留你要用的网盘为“已启用”，然后在下面绑定账号。停用的网盘不接收新任务。已移除的插件可以直接点“安装并启用”，不用额外下载。

也可使用“导入插件”选择仓库 `plugins/` 中的 `.drive-plugin.json` 文件。导入成功会安装并启用该网盘。文件不超过 4 KiB，后端校验格式、唯一平台编号、API 版本和适配器版本；未知字段、未知网盘或不匹配版本会被拒绝。

为兼容升级，三个内置插件首次默认安装并启用。状态独立保存在 SQLite metadata 的 `drive_plugins_v1`，不与上传偏好一起保存。停用和移除保留本机授权配置、任务、历史与远端文件；换账号使用绑定卡片里的“换一个账号”。

已经开始的单个平台上传继续运行，仍可暂停和取消。排队中的平台启动前会再检查开关；已停用的平台记为 `skipped`。手动重试前也检查开关。新任务在创建时选定目标，不会因为后来启用别的插件而额外上传。重新拖入文件会按当前开关补传缺失平台。聚合状态忽略 skipped，全跳过显示已取消，不能误报全部成功。

## 插件格式与边界

```json
{"id":"google","apiVersion":1,"version":"1.0.0"}
```

这是声明式插件包，安装的是当前软件内置的官方适配器，不是任意 JavaScript、DLL 或网络下载的可执行扩展。插件包不携带账号、令牌或密钥。新增其他网盘协议仍需开发适配器并更新软件；不能仅改 manifest 冒充支持新网盘。

注册与状态校验独立位于 `src-tauri/src/plugins.rs`，以 `Arc<dyn DriveAdapter>` 保存适配器；具体协议保留在 `adapters/`。前端管理在 `PluginManager.vue`。目前任务、数据库与认证契约仍显式支持三个已实现的平台，增加新平台必须同时扩展这些契约和迁移测试。

## English

Open **Connect drives → Drive plugins**, enable the drives you want and sign in below. To restore a removed plugin, click **Install and enable**. You can also import a supported `.drive-plugin.json` package from the repository's `plugins/` folder.

Packages are declarative and select trusted adapters bundled with this build. They do not execute scripts or load external binaries. A new drive protocol requires an app update with its adapter implementation.

Disabling or removing a plugin preserves your sign-in settings, upload history and remote files. Active uploads may finish; queued work checks the current setting before starting. Enable the plugin before retrying. New tasks use only their originally selected drives. Drop a file again to fill in missing drives using the current settings.
