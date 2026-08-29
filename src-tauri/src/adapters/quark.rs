use super::{DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest};
use crate::{
    errors::{AppError, AppResult},
    logging::LogService,
    models::{AuthStatus, ShareResult},
    settings::SettingsService,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::{path::PathBuf, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

pub struct QuarkDriveAdapter {
    settings: Arc<SettingsService>,
    log: Arc<LogService>,
    session_id: String,
}

impl QuarkDriveAdapter {
    pub fn new(settings: Arc<SettingsService>, log: Arc<LogService>) -> Self {
        let random = uuid::Uuid::new_v4().simple().to_string();
        Self {
            settings,
            log,
            session_id: format!("{}-{}", Utc::now().timestamp(), &random[..6]),
        }
    }

    pub async fn cli_available(&self) -> bool {
        self.resolve_cli_script().await.is_ok()
    }

    async fn resolve_cli_script(&self) -> AppResult<String> {
        let configured = self.settings.get()?.quark_cli_script.trim().to_string();
        let configured = if configured.is_empty() {
            std::env::var("QUARK_CLI_SCRIPT").unwrap_or_default()
        } else {
            configured
        };
        if !configured.is_empty() {
            let windows_path = PathBuf::from(&configured);
            if windows_path.exists() {
                return self.windows_to_wsl_path(&windows_path).await;
            }
            if configured.starts_with('/') {
                return Ok(configured);
            }
            return Err(AppError::Unavailable(format!(
                "配置的夸克官方 CLI 脚本不存在：{configured}"
            )));
        }

        let output = Command::new("wsl.exe")
            .args([
                "-e",
                "bash",
                "-lc",
                r#"for f in "$HOME/.agents/skills/quarkclouddrive/scripts/quark-drive.cjs" "$HOME/.codex/skills/quarkclouddrive/scripts/quark-drive.cjs" "$HOME/.claude/skills/quarkclouddrive/scripts/quark-drive.cjs"; do if [ -f "$f" ]; then printf '%s' "$f"; exit 0; fi; done; exit 1"#,
            ])
            .output()
            .await
            .map_err(|error| AppError::Unavailable(format!("无法调用 WSL：{error}")))?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
        Err(AppError::Unavailable(
            "未找到夸克官方 quark-drive.cjs；请按 README 在 WSL 安装官方 Skill，并在设置中填写脚本路径"
                .to_string(),
        ))
    }

    pub async fn login(&self) -> AppResult<String> {
        let values = self
            .run_cli(vec!["login".to_string()], None, CancellationToken::new())
            .await?;
        let message = result_message(&values).unwrap_or_else(|| "夸克网盘授权完成".to_string());
        self.log.info("auth", "夸克官方 CLI 授权流程结束");
        Ok(message)
    }

    async fn windows_to_wsl_path(&self, path: &std::path::Path) -> AppResult<String> {
        let output = Command::new("wsl.exe")
            .arg("-e")
            .arg("wslpath")
            .arg("-a")
            .arg("-u")
            .arg(path.as_os_str())
            .output()
            .await
            .map_err(|error| AppError::Unavailable(format!("无法调用 WSL：{error}")))?;
        if !output.status.success() {
            return Err(AppError::Unavailable(
                "无法把 Windows 文件路径转换为 WSL 路径".to_string(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn ensure_remote_dir(&self, remote_dir: &str) -> AppResult<Option<String>> {
        let path = remote_dir.trim_matches('/');
        if path.is_empty() {
            return Ok(None);
        }
        let values = self
            .run_cli(
                vec![
                    "create-folder".to_string(),
                    "--dir-path".to_string(),
                    path.to_string(),
                ],
                None,
                CancellationToken::new(),
            )
            .await?;
        Ok(values.iter().rev().find_map(|value| {
            value
                .get("data")
                .and_then(|data| data.get("fid"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        }))
    }

    async fn run_cli(
        &self,
        mut args: Vec<String>,
        progress: Option<ProgressSink>,
        cancel: CancellationToken,
    ) -> AppResult<Vec<Value>> {
        let script = self.resolve_cli_script().await?;
        let operation = args.first().cloned().unwrap_or_else(|| "命令".to_string());
        args.extend([
            "--session-input".to_string(),
            format!("双盘发货助手执行夸克{operation}"),
            "--session-id".to_string(),
            self.session_id.clone(),
        ]);
        let mut child = Command::new("wsl.exe")
            .args(["-e", "bash", "-lc", "node \"$1\" \"${@:2}\"", "--"])
            .arg(script)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| AppError::Unavailable(format!("夸克官方 CLI 启动失败：{error}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Other("无法读取夸克 CLI 输出".to_string()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Other("无法读取夸克 CLI 错误输出".to_string()))?;
        let stderr_task = tokio::spawn(async move {
            let mut output = String::new();
            let _ = stderr.read_to_string(&mut output).await;
            output
        });
        let mut lines = BufReader::new(stdout).lines();
        let mut values = Vec::new();
        let mut last_progress = (0_u64, 0_u64, 0_f64);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Err(AppError::Paused);
                }
                line = lines.next_line() => {
                    match line? {
                        Some(line) => {
                            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                                if value.get("type").and_then(Value::as_str) == Some("progress") {
                                    if let Some(callback) = &progress {
                                        let data = value.get("data").cloned().unwrap_or(Value::Null);
                                        last_progress = (
                                            data.get("current").and_then(Value::as_u64).unwrap_or(0),
                                            data.get("total").and_then(Value::as_u64).unwrap_or(0),
                                            data.get("percent").and_then(Value::as_f64).unwrap_or(0.0),
                                        );
                                        callback(UploadProgress {
                                            uploaded_bytes: last_progress.0,
                                            total_bytes: last_progress.1,
                                            percent: last_progress.2,
                                            upload_id: None,
                                            resume_data: None,
                                        });
                                    }
                                }
                                if let Some(record_id) = value
                                    .get("data")
                                    .and_then(|data| data.get("recordId"))
                                    .and_then(Value::as_str)
                                {
                                    if let Some(callback) = &progress {
                                        callback(UploadProgress {
                                            uploaded_bytes: last_progress.0,
                                            total_bytes: last_progress.1,
                                            percent: last_progress.2,
                                            upload_id: Some(record_id.to_string()),
                                            resume_data: Some(
                                                serde_json::json!({ "recordId": record_id })
                                                    .to_string(),
                                            ),
                                        });
                                    }
                                }
                                values.push(value);
                            }
                        }
                        None => break,
                    }
                }
            }
        }
        let status = child.wait().await?;
        let stderr_output = stderr_task.await.unwrap_or_default();
        let result_error = values.iter().rev().find_map(|value| {
            let code = value.get("code").and_then(Value::as_i64)?;
            (code != 0 && !(operation == "login" && code == -118)).then(|| {
                value
                    .get("msg")
                    .and_then(Value::as_str)
                    .unwrap_or("夸克官方 CLI 返回错误")
                    .to_string()
            })
        });
        if let Some(message) = result_error {
            return Err(
                if message.contains("未授权") || message.to_ascii_lowercase().contains("token") {
                    AppError::Authentication(message)
                } else {
                    AppError::Provider(message)
                },
            );
        }
        if !status.success() {
            let message = stderr_output
                .lines()
                .next()
                .unwrap_or("夸克官方 CLI 执行失败");
            return Err(AppError::Provider(message.to_string()));
        }
        Ok(values)
    }
}

#[async_trait]
impl DriveAdapter for QuarkDriveAdapter {
    fn provider(&self) -> &'static str {
        "quark"
    }

    async fn authenticate(&self) -> AppResult<AuthStatus> {
        let available = self.cli_available().await;
        if !available {
            return Ok(AuthStatus {
                provider: "quark".to_string(),
                authenticated: false,
                available: false,
                expires_at: None,
                message: "未找到夸克官方 Skill CLI；当前官方仅支持 Windows WSL".to_string(),
            });
        }
        let result = self
            .run_cli(
                vec!["get-user-info".to_string()],
                None,
                CancellationToken::new(),
            )
            .await;
        Ok(AuthStatus {
            provider: "quark".to_string(),
            authenticated: result.is_ok(),
            available: true,
            expires_at: None,
            message: match result {
                Ok(_) => "夸克官方 CLI 已授权".to_string(),
                Err(error) => error.to_string(),
            },
        })
    }

    async fn refresh_token(&self) -> AppResult<()> {
        self.authenticate().await.and_then(|status| {
            if status.authenticated {
                Ok(())
            } else {
                Err(AppError::Authentication(status.message))
            }
        })
    }

    async fn upload_file(
        &self,
        request: UploadRequest,
        progress: ProgressSink,
        cancel: CancellationToken,
    ) -> AppResult<UploadOutcome> {
        self.log
            .info("quark", &format!("开始处理任务 {}", request.task_id));
        let wsl_path = self.windows_to_wsl_path(&request.local_path).await?;
        let resume_record = request.resume_data.as_deref().and_then(|value| {
            serde_json::from_str::<Value>(value)
                .ok()
                .and_then(|json| json.get("recordId")?.as_str().map(ToOwned::to_owned))
        });
        let mut args = if let Some(record_id) = resume_record {
            vec![
                "upload".to_string(),
                "resume".to_string(),
                "--record-id".to_string(),
                record_id,
            ]
        } else {
            let parent_fid = self.ensure_remote_dir(&request.remote_dir).await?;
            let mut upload_args = vec!["upload".to_string(), "--file-path".to_string(), wsl_path];
            if let Some(parent_fid) = parent_fid {
                upload_args.push("--parent-fid".to_string());
                upload_args.push(parent_fid);
            }
            upload_args
        };
        if args.is_empty() {
            args.push("upload".to_string());
        }
        let values = self.run_cli(args, Some(progress), cancel).await?;
        let mut remote_id = None;
        let mut record_id = None;
        let mut instant_upload = false;
        let mut remote_path = format!("{}{}", request.remote_dir, request.file_name);
        for value in &values {
            if let Some(data) = value.get("data") {
                if let Some(file_id) = data
                    .get("fileId")
                    .or_else(|| data.get("fids").and_then(|value| value.get(0)))
                    .and_then(Value::as_str)
                {
                    remote_id = Some(file_id.to_string());
                }
                if let Some(value) = data.get("recordId").and_then(Value::as_str) {
                    record_id = Some(value.to_string());
                }
                instant_upload |= data
                    .get("instantUpload")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if let Some(value) = data.get("fullPath").and_then(Value::as_str) {
                    if !value.is_empty() {
                        remote_path =
                            format!("{}/{}", value.trim_end_matches('/'), request.file_name);
                    }
                }
            }
        }
        let resume_data = record_id
            .as_ref()
            .map(|record_id| serde_json::json!({ "recordId": record_id }).to_string());
        self.log.info("quark", "夸克官方 CLI 上传完成");
        Ok(UploadOutcome {
            remote_path,
            remote_id,
            upload_id: record_id,
            resume_data,
            instant_upload,
        })
    }

    async fn upload_folder(&self, _path: PathBuf) -> AppResult<Vec<UploadOutcome>> {
        Err(AppError::Other(
            "文件夹由任务管理器递归展开后逐文件上传".to_string(),
        ))
    }

    async fn get_upload_progress(&self, _task_id: &str) -> AppResult<Option<UploadProgress>> {
        Ok(None)
    }

    async fn create_share(
        &self,
        outcome: &UploadOutcome,
        period_days: u32,
    ) -> AppResult<ShareResult> {
        let fid = outcome.remote_id.as_deref().ok_or_else(|| {
            AppError::Provider("夸克上传成功但未返回文件 FID，无法自动分享".to_string())
        })?;
        let expired_type = period_to_expired_type(period_days);
        let values = self
            .run_cli(
                vec![
                    "share".to_string(),
                    fid.to_string(),
                    "--url-type".to_string(),
                    "1".to_string(),
                    "--expired-type".to_string(),
                    expired_type.to_string(),
                ],
                None,
                CancellationToken::new(),
            )
            .await?;
        let data = values
            .iter()
            .rev()
            .find_map(|value| value.get("data"))
            .ok_or_else(|| AppError::Provider("夸克分享结果为空".to_string()))?;
        let url = data
            .get("share_url")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Provider("夸克分享结果未包含链接".to_string()))?;
        let expires_at =
            (period_days > 0).then(|| Utc::now().timestamp() + period_days as i64 * 86_400);
        Ok(ShareResult {
            url: Some(url.to_string()),
            code: data
                .get("passcode")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            expires_at,
            share_id: None,
            remote_path: Some(outcome.remote_path.clone()),
            notice: None,
        })
    }

    async fn get_share_result(&self, share: &ShareResult) -> AppResult<ShareResult> {
        Ok(share.clone())
    }

    async fn cancel_upload(&self, _task_id: &str) -> AppResult<()> {
        Ok(())
    }
}

fn result_message(values: &[Value]) -> Option<String> {
    values.iter().rev().find_map(|value| {
        value
            .get("msg")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn period_to_expired_type(days: u32) -> u32 {
    match days {
        0 => 1,
        1 => 2,
        2..=7 => 3,
        8..=30 => 4,
        31..=60 => 5,
        61..=100 => 6,
        _ => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::period_to_expired_type;

    #[test]
    fn share_period_uses_supported_quark_values() {
        assert_eq!(period_to_expired_type(0), 1);
        assert_eq!(period_to_expired_type(30), 4);
        assert_eq!(period_to_expired_type(365), 7);
    }
}
