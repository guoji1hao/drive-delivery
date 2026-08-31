use super::{DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest};
use crate::{
    auth::AuthManager,
    errors::{AppError, AppResult},
    logging::LogService,
    models::{AuthStatus, ShareResult},
};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD_API: &str = "https://www.googleapis.com/upload/drive/v3";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const FILE_MIME: &str = "application/octet-stream";
const CHUNK_SIZE: u64 = 8 * 1024 * 1024;
const MAX_SESSION_AGE_SECONDS: i64 = 6 * 24 * 60 * 60;

pub struct GoogleDriveAdapter {
    auth: Arc<AuthManager>,
    client: Client,
    log: Arc<LogService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleResume {
    session_uri: String,
    uploaded_bytes: u64,
    created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
    id: String,
    #[serde(default)]
    web_view_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Debug, Deserialize)]
struct Permission {
    id: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    role: String,
}

#[derive(Debug, Deserialize)]
struct PermissionList {
    #[serde(default)]
    permissions: Vec<Permission>,
}

enum SessionState {
    Incomplete(u64),
    Complete(DriveFile),
    Expired,
}

impl GoogleDriveAdapter {
    pub fn new(auth: Arc<AuthManager>, log: Arc<LogService>) -> AppResult<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(180))
            .user_agent("dual-drive-assistant/0.2.0")
            .build()?;
        Ok(Self { auth, client, log })
    }

    async fn json_request<T: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        query: &[(String, String)],
        body: Option<&Value>,
        retry_limit: u32,
        operation: &str,
    ) -> AppResult<T> {
        let mut attempt = 0_u32;
        let mut force_refresh = false;
        loop {
            let token = if force_refresh {
                force_refresh = false;
                self.auth.refresh_google_token().await?
            } else {
                self.auth.valid_google_access_token().await?
            };
            let mut request = self
                .client
                .request(method.clone(), url)
                .bearer_auth(&token)
                .query(query);
            if let Some(body) = body {
                request = request.json(body);
            }
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    let response_body = response.text().await?;
                    if status.is_success() {
                        return serde_json::from_str(&response_body).map_err(|error| {
                            AppError::Provider(format!("Google {operation} 响应无法解析：{error}"))
                        });
                    }
                    if status == StatusCode::UNAUTHORIZED && attempt <= retry_limit {
                        force_refresh = true;
                    } else if is_retryable_status(status, &response_body) && attempt < retry_limit {
                        // handled by bounded backoff below
                    } else {
                        return Err(google_api_error(operation, status, &response_body));
                    }
                }
                Err(error)
                    if (error.is_timeout() || error.is_connect()) && attempt < retry_limit =>
                {
                    self.log
                        .info("google", &format!("{operation} 网络中断，将执行有限重试"));
                }
                Err(error) => return Err(error.into()),
            }
            let delay = 1_u64 << attempt.min(5);
            sleep(Duration::from_secs(delay)).await;
            attempt += 1;
        }
    }

    async fn ensure_folder_path(&self, remote_dir: &str, retry_limit: u32) -> AppResult<String> {
        let mut parent_id = "root".to_string();
        for name in remote_dir
            .trim_matches('/')
            .split('/')
            .filter(|part| !part.trim().is_empty())
        {
            let escaped_name = escape_drive_query(name);
            let query_value = format!(
                "name = '{escaped_name}' and mimeType = '{FOLDER_MIME}' and trashed = false and '{parent_id}' in parents"
            );
            let query = vec![
                ("q".to_string(), query_value),
                ("spaces".to_string(), "drive".to_string()),
                ("pageSize".to_string(), "1".to_string()),
                ("fields".to_string(), "files(id,name)".to_string()),
            ];
            let list: FileList = self
                .json_request(
                    Method::GET,
                    &format!("{DRIVE_API}/files"),
                    &query,
                    None,
                    retry_limit,
                    "目录查询",
                )
                .await?;
            if let Some(folder) = list.files.into_iter().next() {
                parent_id = folder.id;
                continue;
            }

            let body = json!({
                "name": name,
                "mimeType": FOLDER_MIME,
                "parents": [parent_id]
            });
            let created: DriveFile = self
                .json_request(
                    Method::POST,
                    &format!("{DRIVE_API}/files"),
                    &[("fields".to_string(), "id,name".to_string())],
                    Some(&body),
                    0,
                    "创建目录",
                )
                .await?;
            parent_id = created.id;
        }
        Ok(parent_id)
    }

    async fn initiate_session(
        &self,
        request: &UploadRequest,
        parent_id: &str,
    ) -> AppResult<GoogleResume> {
        let metadata = json!({
            "name": request.file_name,
            "parents": [parent_id]
        });
        let mut attempt = 0_u32;
        let mut force_refresh = false;
        loop {
            let token = if force_refresh {
                force_refresh = false;
                self.auth.refresh_google_token().await?
            } else {
                self.auth.valid_google_access_token().await?
            };
            let response = self
                .client
                .post(format!("{UPLOAD_API}/files"))
                .query(&[
                    ("uploadType", "resumable"),
                    ("fields", "id,name,webViewLink"),
                ])
                .bearer_auth(token)
                .header("X-Upload-Content-Type", FILE_MIME)
                .header("X-Upload-Content-Length", request.file_size)
                .json(&metadata)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let session_uri = response
                        .headers()
                        .get(header::LOCATION)
                        .and_then(|value| value.to_str().ok())
                        .filter(|value| value.starts_with("https://"))
                        .ok_or_else(|| {
                            AppError::Provider("Google 未返回 resumable upload session".to_string())
                        })?
                        .to_string();
                    return Ok(GoogleResume {
                        session_uri,
                        uploaded_bytes: 0,
                        created_at: Utc::now().timestamp(),
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await?;
                    if status == StatusCode::UNAUTHORIZED && attempt <= request.retry_limit {
                        force_refresh = true;
                    } else if !is_retryable_status(status, &body) || attempt >= request.retry_limit
                    {
                        return Err(google_api_error("初始化上传", status, &body));
                    }
                }
                Err(error)
                    if (error.is_timeout() || error.is_connect())
                        && attempt < request.retry_limit => {}
                Err(error) => return Err(error.into()),
            }
            sleep(Duration::from_secs(1_u64 << attempt.min(5))).await;
            attempt += 1;
        }
    }

    async fn query_session(
        &self,
        resume: &GoogleResume,
        total: u64,
        retry_limit: u32,
    ) -> AppResult<SessionState> {
        let mut attempt = 0_u32;
        let mut force_refresh = false;
        loop {
            let token = if force_refresh {
                force_refresh = false;
                self.auth.refresh_google_token().await?
            } else {
                self.auth.valid_google_access_token().await?
            };
            match self
                .client
                .put(&resume.session_uri)
                .bearer_auth(token)
                .header(header::CONTENT_LENGTH, 0)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    if status == StatusCode::OK || status == StatusCode::CREATED {
                        let file = response.json().await.map_err(|_| {
                            AppError::Provider("Google 上传完成响应无法解析".to_string())
                        })?;
                        return Ok(SessionState::Complete(file));
                    }
                    if status.as_u16() == 308 {
                        return Ok(SessionState::Incomplete(next_offset(response.headers())));
                    }
                    if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
                        return Ok(SessionState::Expired);
                    }
                    let body = response.text().await.unwrap_or_default();
                    if status == StatusCode::UNAUTHORIZED && attempt <= retry_limit {
                        force_refresh = true;
                    } else if !is_retryable_status(status, &body) || attempt >= retry_limit {
                        return Err(google_api_error("查询上传断点", status, &body));
                    }
                }
                Err(error)
                    if (error.is_timeout() || error.is_connect()) && attempt < retry_limit => {}
                Err(error) => {
                    return Err(AppError::Provider(if error.is_timeout() {
                        "Google 上传断点查询超时，可稍后继续".to_string()
                    } else {
                        "Google 上传断点查询网络失败，可稍后继续".to_string()
                    }));
                }
            }
            sleep(Duration::from_secs(1_u64 << attempt.min(5))).await;
            attempt += 1;
        }
    }

    async fn upload_empty_file(
        &self,
        request: &UploadRequest,
        parent_id: &str,
    ) -> AppResult<DriveFile> {
        let boundary = format!("dual-drive-{}", Uuid::new_v4().simple());
        let metadata = serde_json::to_string(&json!({
            "name": request.file_name,
            "parents": [parent_id]
        }))
        .map_err(|error| AppError::Other(error.to_string()))?;
        let body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n--{boundary}\r\nContent-Type: {FILE_MIME}\r\n\r\n\r\n--{boundary}--\r\n"
        );
        let mut attempt = 0_u32;
        let mut force_refresh = false;
        loop {
            let token = if force_refresh {
                force_refresh = false;
                self.auth.refresh_google_token().await?
            } else {
                self.auth.valid_google_access_token().await?
            };
            match self
                .client
                .post(format!("{UPLOAD_API}/files"))
                .query(&[
                    ("uploadType", "multipart"),
                    ("fields", "id,name,webViewLink"),
                ])
                .bearer_auth(token)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/related; boundary={boundary}"),
                )
                .body(body.clone())
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    let response_body = response.text().await?;
                    if status.is_success() {
                        return serde_json::from_str(&response_body).map_err(|error| {
                            AppError::Provider(format!("Google 上传响应无法解析：{error}"))
                        });
                    }
                    if status == StatusCode::UNAUTHORIZED && attempt <= request.retry_limit {
                        force_refresh = true;
                    } else if !is_retryable_status(status, &response_body)
                        || attempt >= request.retry_limit
                    {
                        return Err(google_api_error("上传空文件", status, &response_body));
                    }
                }
                Err(error)
                    if (error.is_timeout() || error.is_connect())
                        && attempt < request.retry_limit => {}
                Err(error) => return Err(error.into()),
            }
            sleep(Duration::from_secs(1_u64 << attempt.min(5))).await;
            attempt += 1;
        }
    }

    async fn upload_chunks(
        &self,
        request: &UploadRequest,
        mut resume: GoogleResume,
        parent_id: &str,
        progress: &ProgressSink,
        cancel: &CancellationToken,
    ) -> AppResult<DriveFile> {
        match self
            .query_session(&resume, request.file_size, request.retry_limit)
            .await
        {
            Ok(SessionState::Complete(file)) => return Ok(file),
            Ok(SessionState::Incomplete(offset)) => resume.uploaded_bytes = offset,
            Ok(SessionState::Expired) => {
                resume = self.initiate_session(request, parent_id).await?;
            }
            Err(error) => return Err(error),
        }
        report_resume(progress, request.file_size, &resume)?;

        loop {
            if cancel.is_cancelled() {
                return Err(AppError::Paused);
            }
            if resume.uploaded_bytes >= request.file_size {
                match self
                    .query_session(&resume, request.file_size, request.retry_limit)
                    .await?
                {
                    SessionState::Complete(file) => return Ok(file),
                    SessionState::Incomplete(offset) => resume.uploaded_bytes = offset,
                    SessionState::Expired => {
                        resume = self.initiate_session(request, parent_id).await?;
                    }
                }
            }

            let start = resume.uploaded_bytes;
            let part_size = (request.file_size - start).min(CHUNK_SIZE);
            let mut file = File::open(&request.local_path).await?;
            file.seek(std::io::SeekFrom::Start(start)).await?;
            let mut buffer = vec![0_u8; part_size as usize];
            file.read_exact(&mut buffer).await?;
            let end = start + part_size - 1;
            let mut attempt = 0_u32;
            let mut force_refresh = false;

            loop {
                if cancel.is_cancelled() {
                    return Err(AppError::Paused);
                }
                let token = if force_refresh {
                    force_refresh = false;
                    self.auth.refresh_google_token().await?
                } else {
                    self.auth.valid_google_access_token().await?
                };
                let response = self
                    .client
                    .put(&resume.session_uri)
                    .bearer_auth(token)
                    .header(header::CONTENT_LENGTH, part_size)
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes {start}-{end}/{}", request.file_size),
                    )
                    .body(buffer.clone())
                    .send()
                    .await;
                match response {
                    Ok(response) => {
                        let status = response.status();
                        if status == StatusCode::OK || status == StatusCode::CREATED {
                            let file: DriveFile = response.json().await.map_err(|_| {
                                AppError::Provider("Google 上传完成响应无法解析".to_string())
                            })?;
                            progress(UploadProgress {
                                uploaded_bytes: request.file_size,
                                total_bytes: request.file_size,
                                percent: 100.0,
                                upload_id: None,
                                resume_data: None,
                            });
                            return Ok(file);
                        }
                        if status.as_u16() == 308 {
                            resume.uploaded_bytes = next_offset(response.headers());
                            report_resume(progress, request.file_size, &resume)?;
                            break;
                        }
                        if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
                            resume = self.initiate_session(request, parent_id).await?;
                            report_resume(progress, request.file_size, &resume)?;
                            break;
                        }
                        let body = response.text().await.unwrap_or_default();
                        if status == StatusCode::UNAUTHORIZED && attempt <= request.retry_limit {
                            force_refresh = true;
                        } else if is_retryable_status(status, &body)
                            && attempt < request.retry_limit
                        {
                            // bounded backoff below
                        } else {
                            return Err(google_api_error("上传分片", status, &body));
                        }
                    }
                    Err(error)
                        if (error.is_timeout() || error.is_connect())
                            && attempt < request.retry_limit => {}
                    Err(error) => {
                        return Err(AppError::Provider(if error.is_timeout() {
                            "Google 上传分片超时，已保存断点".to_string()
                        } else {
                            "Google 上传分片网络失败，已保存断点".to_string()
                        }));
                    }
                }
                let delay = 1_u64 << attempt.min(5);
                self.log
                    .info("google", &format!("上传分片暂时失败，{delay} 秒后有限重试"));
                tokio::select! {
                    _ = sleep(Duration::from_secs(delay)) => {},
                    _ = cancel.cancelled() => return Err(AppError::Paused),
                }
                attempt += 1;
            }
        }
    }

    async fn public_permission(
        &self,
        file_id: &str,
        retry_limit: u32,
    ) -> AppResult<Option<Permission>> {
        let query = vec![(
            "fields".to_string(),
            "permissions(id,type,role)".to_string(),
        )];
        let list: PermissionList = self
            .json_request(
                Method::GET,
                &format!("{DRIVE_API}/files/{file_id}/permissions"),
                &query,
                None,
                retry_limit,
                "查询分享权限",
            )
            .await?;
        Ok(list
            .permissions
            .into_iter()
            .find(|item| item.r#type == "anyone" && item.role == "reader"))
    }
}

#[async_trait]
impl DriveAdapter for GoogleDriveAdapter {
    fn provider(&self) -> &'static str {
        "google"
    }

    async fn authenticate(&self) -> AppResult<AuthStatus> {
        Ok(self.auth.google_status())
    }

    async fn refresh_token(&self) -> AppResult<()> {
        self.auth.refresh_google_token().await.map(|_| ())
    }

    async fn upload_file(
        &self,
        request: UploadRequest,
        progress: ProgressSink,
        cancel: CancellationToken,
    ) -> AppResult<UploadOutcome> {
        if !request.local_path.is_file() {
            return Err(AppError::File("文件不存在或已被移动".to_string()));
        }
        self.log
            .info("google", &format!("开始处理任务 {}", request.task_id));
        let parent_id = self
            .ensure_folder_path(&request.remote_dir, request.retry_limit)
            .await?;
        let file = if request.file_size == 0 {
            let file = self.upload_empty_file(&request, &parent_id).await?;
            progress(UploadProgress {
                uploaded_bytes: 0,
                total_bytes: 0,
                percent: 100.0,
                upload_id: None,
                resume_data: None,
            });
            file
        } else {
            let resume = request
                .resume_data
                .as_deref()
                .and_then(|value| serde_json::from_str::<GoogleResume>(value).ok())
                .filter(|value| {
                    value.created_at + MAX_SESSION_AGE_SECONDS > Utc::now().timestamp()
                        && value.session_uri.starts_with("https://")
                });
            let resume = match resume {
                Some(resume) => resume,
                None => self.initiate_session(&request, &parent_id).await?,
            };
            self.upload_chunks(&request, resume, &parent_id, &progress, &cancel)
                .await?
        };
        self.log.info("google", "Google Drive 文件上传完成");
        Ok(UploadOutcome {
            remote_path: format!(
                "My Drive/{}{}",
                request.remote_dir.trim_matches('/'),
                if request.remote_dir.trim_matches('/').is_empty() {
                    request.file_name.clone()
                } else {
                    format!("/{}", request.file_name)
                }
            ),
            remote_id: Some(file.id),
            upload_id: None,
            resume_data: None,
            instant_upload: false,
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
        _period_days: u32,
    ) -> AppResult<ShareResult> {
        let file_id = outcome.remote_id.as_deref().ok_or_else(|| {
            AppError::Provider("Google 上传结果缺少 fileId，无法创建分享".to_string())
        })?;
        let permission = match self.public_permission(file_id, 3).await? {
            Some(permission) => permission,
            None => {
                let body = json!({
                    "type": "anyone",
                    "role": "reader"
                });
                self.json_request(
                    Method::POST,
                    &format!("{DRIVE_API}/files/{file_id}/permissions"),
                    &[("fields".to_string(), "id,type,role".to_string())],
                    Some(&body),
                    0,
                    "创建公开分享",
                )
                .await?
            }
        };
        let file: DriveFile = self
            .json_request(
                Method::GET,
                &format!("{DRIVE_API}/files/{file_id}"),
                &[("fields".to_string(), "id,name,webViewLink".to_string())],
                None,
                3,
                "读取分享链接",
            )
            .await?;
        let url = file
            .web_view_link
            .ok_or_else(|| AppError::Provider("Google Drive 未返回 webViewLink".to_string()))?;
        self.log.info("google", "Google Drive 公开只读分享已创建");
        Ok(ShareResult {
            url: Some(url),
            code: None,
            expires_at: None,
            share_id: Some(permission.id),
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

fn report_resume(progress: &ProgressSink, total: u64, resume: &GoogleResume) -> AppResult<()> {
    let resume_data = serde_json::to_string(resume)
        .map_err(|error| AppError::Other(format!("保存 Google 断点失败：{error}")))?;
    progress(UploadProgress {
        uploaded_bytes: resume.uploaded_bytes.min(total),
        total_bytes: total,
        percent: if total == 0 {
            100.0
        } else {
            resume.uploaded_bytes.min(total) as f64 * 100.0 / total as f64
        },
        upload_id: None,
        resume_data: Some(resume_data),
    });
    Ok(())
}

fn next_offset(headers: &header::HeaderMap) -> u64 {
    headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit('-').next())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|last| last.saturating_add(1))
        .unwrap_or(0)
}

fn escape_drive_query(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn is_retryable_status(status: StatusCode, body: &str) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
        || (status == StatusCode::FORBIDDEN
            && (body.contains("rateLimitExceeded") || body.contains("userRateLimitExceeded")))
}

fn google_api_error(operation: &str, status: StatusCode, body: &str) -> AppError {
    if status == StatusCode::UNAUTHORIZED {
        return AppError::Authentication(
            "Google Token 已失效，自动刷新失败，请重新授权".to_string(),
        );
    }
    if status == StatusCode::FORBIDDEN {
        if body.contains("storageQuotaExceeded") {
            return AppError::Provider("Google Drive 存储空间不足".to_string());
        }
        return AppError::Provider(format!(
            "Google {operation} 被拒绝；请检查 Drive API、OAuth 权限或组织分享策略"
        ));
    }
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("HTTP {status}"));
    AppError::Provider(format!("Google {operation} 失败：{message}"))
}

#[cfg(test)]
mod tests {
    use super::{CHUNK_SIZE, escape_drive_query, next_offset};
    use reqwest::header::{HeaderMap, HeaderValue, RANGE};

    #[test]
    fn chunks_follow_drive_alignment_rule() {
        assert_eq!(CHUNK_SIZE % (256 * 1024), 0);
    }

    #[test]
    fn parses_server_confirmed_range() {
        let mut headers = HeaderMap::new();
        headers.insert(RANGE, HeaderValue::from_static("bytes=0-524287"));
        assert_eq!(next_offset(&headers), 524288);
    }

    #[test]
    fn escapes_folder_search_literals() {
        assert_eq!(escape_drive_query("客户's\\资料"), "客户\\'s\\\\资料");
    }
}
