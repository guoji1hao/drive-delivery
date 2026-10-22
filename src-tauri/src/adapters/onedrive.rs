use super::{DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest};
use crate::{
    auth::AuthManager,
    errors::{AppError, AppResult},
    models::{AuthStatus, ShareResult},
};
use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::sync::CancellationToken;

const GRAPH: &str = "https://graph.microsoft.com/v1.0/me/drive";
const CHUNK: u64 = 5 * 1024 * 1024; // 16 × 320 KiB, as required by Graph.

pub struct OneDriveAdapter {
    auth: Arc<AuthManager>,
    client: Client,
}

#[derive(Serialize, Deserialize)]
struct Resume {
    session_url: String,
    item_path: String,
    drive_id: String,
}

impl OneDriveAdapter {
    pub fn new(auth: Arc<AuthManager>) -> AppResult<Self> {
        Ok(Self {
            auth,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(180))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }

    async fn graph(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> AppResult<reqwest::Response> {
        let mut token = self.auth.valid_onedrive_access_token().await?;
        for attempt in 0..2 {
            let mut request = self
                .client
                .request(method.clone(), format!("{GRAPH}{path}"))
                .bearer_auth(&token);
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request.send().await.map_err(network_error)?;
            if response.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                token = self.auth.refresh_onedrive_token().await?;
                continue;
            }
            return Ok(response);
        }
        unreachable!()
    }

    async fn folder(&self, directory: &str) -> AppResult<String> {
        let root = checked(self.graph(Method::GET, "/root", None).await?).await?;
        let mut parent = required(&root, "id")?.to_string();
        for name in directory.split('/').filter(|s| !s.is_empty()) {
            validate_name(name)?;
            let path = format!("/items/{}:/{}", enc(&parent), enc(name));
            let response = self.graph(Method::GET, &path, None).await?;
            let item = if response.status() == StatusCode::NOT_FOUND {
                let response = self.graph(Method::POST, &format!("/items/{}/children", enc(&parent)),
                    Some(&json!({"name": name, "folder": {}, "@microsoft.graph.conflictBehavior": "fail"}))).await?;
                if response.status() == StatusCode::CONFLICT {
                    checked(self.graph(Method::GET, &path, None).await?).await?
                } else {
                    checked(response).await?
                }
            } else {
                checked(response).await?
            };
            if !item["folder"].is_object() {
                return Err(AppError::Configuration(
                    "OneDrive 目标目录被同名文件占用".into(),
                ));
            }
            parent = required(&item, "id")?.to_string();
        }
        Ok(parent)
    }

    async fn existing(&self, path: &str, size: u64) -> AppResult<Option<Value>> {
        let response = self.graph(Method::GET, path, None).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let item = checked(response).await?;
        if item["size"].as_u64() != Some(size) || !item["file"].is_object() {
            return Err(AppError::Configuration(
                "OneDrive 同名结果大小不匹配，已停止以避免覆盖".into(),
            ));
        }
        Ok(Some(item))
    }
}

#[async_trait]
impl DriveAdapter for OneDriveAdapter {
    fn provider(&self) -> &'static str {
        "onedrive"
    }
    async fn authenticate(&self) -> AppResult<AuthStatus> {
        Ok(self.auth.onedrive_status())
    }
    async fn refresh_token(&self) -> AppResult<()> {
        self.auth.refresh_onedrive_token().await.map(|_| ())
    }
    async fn ensure_directory(
        &self,
        path: &str,
        _retry_limit: u32,
        cancel: &CancellationToken,
    ) -> AppResult<String> {
        if cancel.is_cancelled() {
            return Err(AppError::Paused);
        }
        let id = self.folder(path).await?;
        if cancel.is_cancelled() {
            return Err(AppError::Paused);
        }
        Ok(id)
    }
    async fn upload_file(
        &self,
        request: UploadRequest,
        progress: ProgressSink,
        cancel: CancellationToken,
    ) -> AppResult<UploadOutcome> {
        if cancel.is_cancelled() {
            return Err(AppError::Paused);
        }
        validate_name(&request.file_name)?;
        let drive = checked(self.graph(Method::GET, "", None).await?).await?;
        let drive_id = required(&drive, "id")?.to_string();
        let mut resume: Option<Resume> = request
            .resume_data
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|_| AppError::Configuration("OneDrive 续传记录损坏".into()))?;
        if let Some(saved) = &resume {
            if saved.drive_id != drive_id {
                return Err(AppError::Configuration(
                    "OneDrive 账号已改变，请切回原账号继续此任务".into(),
                ));
            }
            validate_session(&saved.session_url)?;
        }
        // A stable task suffix makes lost completion responses recoverable without overwriting user files.
        let name = remote_name(&request.file_name, &request.task_id);
        let item_path = if let Some(saved) = &resume {
            saved.item_path.clone()
        } else {
            let parent = self.folder(&request.remote_dir).await?;
            format!("/items/{}:/{}", enc(&parent), enc(&name))
        };
        let remote_path = format!("OneDrive{}{}", request.remote_dir, name);
        if let Some(item) = self.existing(&item_path, request.file_size).await? {
            return outcome(&item, remote_path, true);
        }
        if request.file_size == 0 {
            if cancel.is_cancelled() {
                return Err(AppError::Paused);
            }
            let token = self.auth.valid_onedrive_access_token().await?;
            let response = self
                .client
                .put(format!("{GRAPH}{item_path}:/content"))
                .query(&[("@microsoft.graph.conflictBehavior", "fail")])
                .bearer_auth(token)
                .header("Content-Type", "application/octet-stream")
                .body(Vec::new())
                .send()
                .await
                .map_err(network_error)?;
            return outcome(&checked(response).await?, remote_path, false);
        }
        let mut offset = 0;
        if let Some(saved) = &resume {
            let response = self
                .client
                .get(&saved.session_url)
                .send()
                .await
                .map_err(network_error)?;
            if response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::GONE {
                // Recheck after session expiry: the final PUT may have committed before its response was lost.
                if let Some(item) = self.existing(&item_path, request.file_size).await? {
                    return outcome(&item, remote_path, true);
                }
                resume = None;
            } else {
                offset = next_offset(&checked(response).await?, request.file_size)?;
            }
        }
        if resume.is_none() {
            if cancel.is_cancelled() {
                return Err(AppError::Paused);
            }
            let session = checked(self.graph(Method::POST, &format!("{item_path}:/createUploadSession"),
                Some(&json!({"item": {"name": name, "@microsoft.graph.conflictBehavior": "fail"}}))).await?).await?;
            let session_url = required(&session, "uploadUrl")?.to_string();
            validate_session(&session_url)?;
            resume = Some(Resume {
                session_url,
                item_path,
                drive_id,
            });
        }
        let resume = resume.expect("session created");
        let encoded = serde_json::to_string(&resume)
            .map_err(|_| AppError::Other("无法保存 OneDrive 断点".into()))?;
        let emit = |bytes: u64| {
            progress(UploadProgress {
                uploaded_bytes: bytes,
                total_bytes: request.file_size,
                percent: bytes as f64 / request.file_size as f64 * 100.0,
                upload_id: None,
                resume_data: Some(encoded.clone()),
            })
        };
        emit(offset);
        let mut file = File::open(&request.local_path).await?;
        while offset < request.file_size {
            if cancel.is_cancelled() {
                return Err(AppError::Paused);
            }
            let count = (request.file_size - offset).min(CHUNK) as usize;
            file.seek(std::io::SeekFrom::Start(offset)).await?;
            let mut bytes = vec![0; count];
            file.read_exact(&mut bytes).await?;
            // The preauthenticated upload URL must never receive a Graph bearer token.
            let response = tokio::select! {
                _ = cancel.cancelled() => return Err(AppError::Paused),
                result = self.client.put(&resume.session_url)
                    .header("Content-Length", count)
                    .header("Content-Range", format!("bytes {}-{}/{}", offset, offset + count as u64 - 1, request.file_size))
                    .body(bytes).send() => result.map_err(network_error)?,
            };
            let status = response.status();
            let value = checked(response).await?;
            if status == StatusCode::OK || status == StatusCode::CREATED {
                if value["size"].as_u64() != Some(request.file_size) {
                    return Err(AppError::Provider(
                        "OneDrive 返回的完成大小不匹配，保留断点等待重试".into(),
                    ));
                }
                return outcome(&value, remote_path, false);
            }
            let next = next_offset(&value, request.file_size)?;
            if next <= offset {
                return Err(AppError::Provider(
                    "OneDrive 上传未推进，请重试查询服务端断点".into(),
                ));
            }
            offset = next;
            emit(offset);
        }
        Err(AppError::Provider(
            "OneDrive 尚未确认文件提交，保留断点等待重试".into(),
        ))
    }

    async fn create_share(
        &self,
        outcome: &UploadOutcome,
        period_days: u32,
    ) -> AppResult<ShareResult> {
        let id = outcome
            .remote_id
            .as_deref()
            .ok_or_else(|| AppError::Configuration("缺少 OneDrive 文件 ID".into()))?;
        let mut body = json!({"type": "view", "scope": "anonymous"});
        if period_days > 0 {
            body["expirationDateTime"] = json!(
                (chrono::Utc::now() + chrono::Duration::days(period_days as i64)).to_rfc3339()
            );
        }
        let value = checked(
            self.graph(
                Method::POST,
                &format!("/items/{}/createLink", enc(id)),
                Some(&body),
            )
            .await?,
        )
        .await?;
        let url = required(&value["link"], "webUrl")?.to_string();
        if !url.starts_with("https://") {
            return Err(AppError::Provider("OneDrive 返回无效分享链接".into()));
        }
        Ok(ShareResult {
            url: Some(url),
            share_id: value["id"].as_str().map(str::to_string),
            expires_at: value["expirationDateTime"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp()),
            remote_path: Some(outcome.remote_path.clone()),
            code: None,
            notice: Some("只读公开链接；有效期以微软返回值及组织策略为准".into()),
        })
    }
    async fn upload_folder(&self, _: PathBuf) -> AppResult<Vec<UploadOutcome>> {
        Err(AppError::Configuration("文件夹由任务管理器展开".into()))
    }
    async fn get_upload_progress(&self, _: &str) -> AppResult<Option<UploadProgress>> {
        Ok(None)
    }
    async fn get_share_result(&self, share: &ShareResult) -> AppResult<ShareResult> {
        Ok(share.clone())
    }
    async fn cancel_upload(&self, _: &str) -> AppResult<()> {
        Ok(())
    }
}

fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}
fn network_error(_: reqwest::Error) -> AppError {
    AppError::Provider("OneDrive 网络请求失败，请重试；已保留断点".into())
}
async fn checked(response: reqwest::Response) -> AppResult<Value> {
    let status = response.status();
    if !status.is_success() {
        let message = format!("OneDrive HTTP {status}；请检查账号空间、文件权限及组织分享策略");
        return Err(if status == StatusCode::UNAUTHORIZED {
            AppError::Authentication(message)
        } else if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            AppError::Provider(message)
        } else {
            AppError::Configuration(message)
        });
    }
    response.json().await.map_err(network_error)
}
fn required<'a>(value: &'a Value, key: &str) -> AppResult<&'a str> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Provider(format!("OneDrive 响应缺少 {key}")))
}
fn outcome(item: &Value, remote_path: String, instant_upload: bool) -> AppResult<UploadOutcome> {
    Ok(UploadOutcome {
        remote_path,
        remote_id: Some(required(item, "id")?.into()),
        upload_id: None,
        resume_data: None,
        instant_upload,
    })
}
fn next_offset(value: &Value, total: u64) -> AppResult<u64> {
    value["nextExpectedRanges"]
        .as_array()
        .and_then(|ranges| {
            ranges
                .iter()
                .map(|r| r.as_str()?.split('-').next()?.parse::<u64>().ok())
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .min()
        })
        .filter(|offset| *offset < total && offset % (320 * 1024) == 0)
        .ok_or_else(|| AppError::Provider("OneDrive 续传范围无效，已保留原会话".into()))
}
fn validate_session(value: &str) -> AppResult<()> {
    let url = url::Url::parse(value)
        .map_err(|_| AppError::Configuration("OneDrive 续传地址无效".into()))?;
    let host = url.host_str().unwrap_or_default();
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
        || ![
            ".up.1drv.com",
            ".up.1drv.com.cn",
            ".sharepoint.com",
            ".onedrive.com",
        ]
        .iter()
        .any(|suffix| host.ends_with(suffix))
    {
        return Err(AppError::Configuration(
            "OneDrive 续传地址不属于支持的微软上传服务".into(),
        ));
    }
    Ok(())
}
fn validate_name(name: &str) -> AppResult<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "\"*:<>?/\\|".contains(c))
    {
        return Err(AppError::Configuration(
            "OneDrive 文件或目录名包含不支持的字符".into(),
        ));
    }
    Ok(())
}
fn remote_name(name: &str, task_id: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => format!("{stem}-{task_id}.{extension}"),
        _ => format!("{name}-{task_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_server_offsets_and_session_origin() {
        assert_eq!(CHUNK % (320 * 1024), 0);
        assert_eq!(
            next_offset(&json!({"nextExpectedRanges": ["5242880-"]}), CHUNK * 2).unwrap(),
            CHUNK
        );
        assert!(next_offset(&json!({"nextExpectedRanges": ["3-"]}), 10).is_err());
        assert!(next_offset(&json!({}), 10).is_err());
        assert!(validate_session("https://sn123.up.1drv.com/session?secret=hidden").is_ok());
        assert!(validate_session("https://evil.test/session").is_err());
        assert!(validate_session("https://sn.up.1drv.com.evil.test/session").is_err());
        assert!(validate_session("http://sn.up.1drv.com/session").is_err());
    }
    #[test]
    fn names_are_safe_and_stable() {
        assert_eq!(remote_name("中文.zip", "task-1"), "中文-task-1.zip");
        assert!(validate_name("..").is_err());
        assert!(validate_name("目录").is_ok());
    }
}
