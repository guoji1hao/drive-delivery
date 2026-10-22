# Browser edition verification

Version 0.1.0, 2026-10-22. Built separately with `npm run extension:build`; output `dist-extension` is the unpacked MV3 extension. Source modules separate authentication, scheduling, file enumeration and each provider protocol. MIT license; spark-md5 retains its upstream license in the package.

`npm run extension:test` exercises queue concurrency/FIFO/pause/retry, Unicode and empty directories, OAuth state/callback validation and PKCE, provider URL allowlists, Google server-offset recovery and lost completion response, OneDrive preauthorized upload without Bearer and share-only retry, Baidu full hash commit after partial precreate, and failure retention of unknown upload sessions.

`npm test` and `npm run build` check the existing frontend. These tests use protocol fixtures, not live accounts. Real provider consent, OAuth app approval, organizational policies, Chrome/Edge native extension execution and actual cloud file round trips remain to be verified. The package is not store-reviewed.

Google uses Chrome identity with drive.file. OneDrive uses authorization-code + S256 PKCE and a registered SPA redirect. Baidu uses personal device authorization. Never bundle shared client secrets. Keep the uploader open; restart resume is intentionally unsupported. A fixed public manifest key stabilizes unpacked identity, not a signing credential.

Official references: [Chrome identity](https://developer.chrome.com/docs/extensions/reference/api/identity), [load unpacked](https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world), [Google uploads](https://developers.google.com/workspace/drive/api/guides/manage-uploads), [Microsoft PKCE](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow), [OneDrive sessions](https://learn.microsoft.com/en-us/graph/api/driveitem-createuploadsession), [Baidu SDK](https://github.com/baidu-netdisk/baidu-drive-sdk-go).
