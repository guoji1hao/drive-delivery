# Drive Delivery

[中文说明](README.md)

A local Windows desktop app for delivering files through official Baidu Drive, Google Drive and OneDrive adapters. Built with Tauri 2, Vue 3, TypeScript, Rust and SQLite.

Choose **English** at the top of the app. The choice is remembered. The installer also offers English and Simplified Chinese. File names, paths and custom copy templates stay unchanged; default share text follows your language. Raw provider diagnostics may remain in their original language.

## Connect your drives

Open **Connect drives**. Under **Drive plugins**, enable the drives you want and disable the others. Click **Connect** on a drive, sign in on its official website and approve access. Return to the app and wait for **Connected**. You only need to do this once.

If you see **First-time setup**, the app needs that provider's OAuth configuration. Ask your app provider for it or follow the built-in guide. For Google, import a Desktop app JSON file from Google Cloud Console. Do not enter your drive password in the app. This repository does not ship public OAuth credentials; a distributor must register and prepare them before offering ready-to-use sign-in.

For device-code sign-in, paste the displayed code into the official website. Use **Reopen sign-in page** if you close it accidentally. Switching app pages does not stop sign-in polling.

## Upload and share

Drop files or folders onto **Uploads**. Each enabled drive runs independently, with progress, pause, cancel, resume and retry. Local SHA-256 history avoids uploading the same file again and can recreate expired links without uploading again. Completed results remain available under **History**.

Baidu supports official uploads, but automatic public sharing is not confirmed for its personal API. Create its public links manually. Google public links have no access code or configured expiry. Organization policies may block public sharing on Google Workspace or OneDrive.

## Plugins

Built-in plugins can be installed, enabled, disabled, removed and restored. Import packages from `plugins/` if needed. These are declarative packages for adapters bundled with this app, not executable third-party extensions. Other drive protocols require a new adapter implementation and app update. See [plugin contract and lifecycle](docs/DRIVE_PLUGINS.md).

OAuth credentials and tokens use the operating system's secure credential store. Upload tasks, history, plugin state and settings are local SQLite data. Removing a plugin does not delete remote files or local history.

## Development

On Windows with Node.js, Rust, WebView2 and Visual Studio C++ Build Tools:

```powershell
npm.cmd install
npm.cmd test
npm.cmd run build
cargo test --release --manifest-path src-tauri/Cargo.toml
npm.cmd run tauri build -- --bundles nsis
```

Run Rust commands in a Visual Studio developer shell. Browser smoke scripts in `scripts/` mock Tauri IPC. They verify UI behavior, not live provider authorization or uploads. See [verification notes](docs/VERIFICATION.md).
