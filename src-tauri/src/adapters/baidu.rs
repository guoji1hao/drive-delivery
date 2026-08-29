use super::{DriveAdapter, ProgressSink, UploadOutcome, UploadProgress, UploadRequest};
use crate::{
    auth::AuthManager,
    errors::{AppError, AppResult},
    logging::LogService,
    models::{AuthStatus, ShareResult},
};
use async_trait::async_trait;
use md5::{Digest as Md5Digest, Md5};
use reqwest::{Client, StatusCode, multipart};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::BTreeMap, io::Read, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    time::sleep,
};
use tokio_util::sync::CancellationToken;

const SLICE_SIZE: u64 = 4 * 1024 * 1024;
const FILE_API: &str = "https://pan.baidu.com/rest/2.0/xpan/file";
const SLICE_API: &str = "https://d.pcs.baidu.com/rest/2.0/pcs/superfile2";

pub struct BaiduDriveAdapter {
    auth: Arc<AuthManager>,
    client: Client,
    log: Arc<LogService>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BaiduResume {
    block_md5s: Vec<String>,
    upload_id: String,
    needed_parts: Vec<usize>,
    completed_parts: BTreeMap<usize, String>,
}

#[derive(Debug, Deserialize)]
struct PrecreateResponse {
    #[serde(default)]
    errno: i64,
    #[serde(default)]
    uploadid: String,
    #[serde(default)]
    block_list: Vec<usize>,
    #[serde(default)]
    return_type: i32,
    #[serde(default)]
    errmsg: String,
}

#[derive(Debug, Deserialize)]
struct SliceResponse {
    #[serde(default)]
    errno: i64,
    #[serde(default)]
    md5: String,
    #[serde(default)]
    errmsg: String,
}

#[derive(Debug, Deserialize)]
struct CreateResponse {
    #[serde(default)]
    errno: i64,
    #[serde(default)]
    fs_id: i64,
    #[serde(default)]
    path: String,
    #[serde(default)]
    errmsg: String,
}

impl BaiduDriveAdapter {
    pub fn new(auth: Arc<AuthManager>, log: Arc<LogService>) -> AppResult<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(180))
            .user_agent("dual-drive-assistant/0.1.0")
            .build()?;
        Ok(Self { auth, client, log })
    }

    async fn compute_block_md5s(path: PathBuf) -> AppResult<Vec<String>> {
        tokio::task::spawn_blocking(move || {
            let mut file = std::fs::File::open(path)?;
            let mut buffer = vec![0_u8; SLICE_SIZE as usize];
            let mut hashes = Vec::new();
            loop {
                let mut read_total = 0;
                while read_total < buffer.len() {
                    let count = file.read(&mut buffer[read_total..])?;
                    if count == 0 {
                        break;
                    }
                    read_total += count;
                }
                if read_total == 0 {
                    break;
                }
                let mut hasher = Md5::new();
                hasher.update(&buffer[..read_total]);
                hashes.push(format!("{:x}", hasher.finalize()));
                if read_total < buffer.len() {
                    break;
                }
            }
            Ok::<_, std::io::Error>(hashes)
        })
        .await
        .map_err(|error| AppError::File(format!("计算百度分片 Hash 失败：{error}")))?
        .map_err(Into::into)
    }

    async fn post_form<T: DeserializeOwned>(
        &self,
        token: &str,
        method: &str,
        form: &[(String, String)],
        retry_limit: u32,
        cancel: &CancellationToken,
    ) -> AppResult<T> {
        let mut attempt = 0;
        loop {
            if cancel.is_cancelled() {
                return Err(AppError::Paused);
            }
            let response = self
                .client
                .post(FILE_API)
                .query(&[("method", method), ("access_token", token)])
                .form(form)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    return response.json::<T>().await.map_err(Into::into);
                }
                Ok(response)
                    if response.status() == StatusCode::TOO_MANY_REQUESTS
                        || response.status().is_server_error() =>
                {
                    if attempt >= retry_limit {
                        return Err(AppError::Provider(format!(
                            "百度接口 HTTP {}，已达到重试上限",
                            response.status()
                        )));
                    }
                }
                Ok(response)
                    if response.status() == StatusCode::UNAUTHORIZED
                        || response.status() == StatusCode::FORBIDDEN =>
                {
                    return Err(AppError::Authentication(
                        "百度 Token 已失效或权限不足，请重新授权".to_string(),
                    ));
                }
                Ok(response) => {
                    return Err(AppError::Provider(format!(
                        "百度接口 HTTP {}",
                        response.status()
                    )));
                }
                Err(error) if error.is_timeout() || error.is_connect() => {
                    if attempt >= retry_limit {
                        return Err(AppError::Provider(format!(
                            "百度接口网络异常，已达到重试上限：{error}"
                        )));
                    }
                }
                Err(error) => return Err(error.into()),
            }
            let delay = 1_u64 << attempt.min(5);
            self.log.info(
                "baidu",
                &format!("请求暂时失败，{delay} 秒后进行第 {} 次重试", attempt + 1),
            );
            tokio::select! {
                _ = sleep(Duration::from_secs(delay)) => {},
                _ = cancel.cancelled() => return Err(AppError::Paused),
            }
            attempt += 1;
        }
    }

    async fn upload_slice(
        &self,
        token: &str,
        request: &UploadRequest,
        remote_path: &str,
        upload_id: &str,
        part_index: usize,
        retry_limit: u32,
        cancel: &CancellationToken,
    ) -> AppResult<SliceResponse> {
        let offset = part_index as u64 * SLICE_SIZE;
        let part_size = request.file_size.saturating_sub(offset).min(SLICE_SIZE);
        let mut attempt = 0;
        loop {
            if cancel.is_cancelled() {
                return Err(AppError::Paused);
            }
            let mut file = File::open(&request.local_path).await?;
            file.seek(std::io::SeekFrom::Start(offset)).await?;
            let mut buffer = vec![0_u8; part_size as usize];
            file.read_exact(&mut buffer).await?;
            let part = multipart::Part::bytes(buffer).file_name("file");
            let form = multipart::Form::new().part("file", part);
            let response = self
                .client
                .post(SLICE_API)
                .query(&[
                    ("method", "upload"),
                    ("type", "tmpfile"),
                    ("path", remote_path),
                    ("uploadid", upload_id),
                    ("partseq", &part_index.to_string()),
                    ("access_token", token),
                ])
                .multipart(form)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let value = response.json::<SliceResponse>().await?;
                    if value.errno == 0 && !value.md5.is_empty() {
                        return Ok(value);
                    }
                    return Err(api_error(value.errno, &value.errmsg));
                }
                Ok(response)
                    if response.status() == StatusCode::TOO_MANY_REQUESTS
                        || response.status().is_server_error() =>
                {
                    if attempt >= retry_limit {
                        return Err(AppError::Provider(format!(
                            "百度分片上传 HTTP {}，已达到重试上限",
                            response.status()
                        )));
                    }
                }
                Ok(response) => {
                    return Err(AppError::Provider(format!(
                        "百度分片上传 HTTP {}",
                        response.status()
                    )));
                }
                Err(error) if error.is_timeout() || error.is_connect() => {
                    if attempt >= retry_limit {
                        return Err(AppError::Provider(format!(
                            "百度分片上传中断，已达到重试上限：{error}"
                        )));
                    }
                }
                Err(error) => return Err(error.into()),
            }
            let delay = 1_u64 << attempt.min(5);
            tokio::select! {
                _ = sleep(Duration::from_secs(delay)) => {},
                _ = cancel.cancelled() => return Err(AppError::Paused),
            }
            attempt += 1;
        }
    }
}

#[async_trait]
impl DriveAdapter for BaiduDriveAdapter {
    fn provider(&self) -> &'static str {
        "baidu"
    }

    async fn authenticate(&self) -> AppResult<AuthStatus> {
        Ok(self.auth.baidu_status())
    }

    async fn refresh_token(&self) -> AppResult<()> {
        self.auth.refresh_baidu_token().await.map(|_| ())
    }

    async fn upload_file(
        &self,
        request: UploadRequest,
        progress: ProgressSink,
        cancel: CancellationToken,
    ) -> AppResult<UploadOutcome> {
        self.log
            .info("baidu", &format!("开始处理任务 {}", request.task_id));
        if !request.local_path.is_file() {
            return Err(AppError::File("文件不存在或已被移动".to_string()));
        }
        let token = self.auth.valid_baidu_access_token().await?;
        let remote_path = format!("{}{}", request.remote_dir, request.file_name);
        let mut resume = request
            .resume_data
            .as_deref()
            .and_then(|value| serde_json::from_str::<BaiduResume>(value).ok())
            .unwrap_or_default();

        if resume.block_md5s.is_empty() {
            resume.block_md5s = Self::compute_block_md5s(request.local_path.clone()).await?;
        }
        if resume.block_md5s.is_empty() {
            return Err(AppError::File("百度开放 API 不接受空文件上传".to_string()));
        }
        if resume.upload_id.is_empty() {
            let block_list = serde_json::to_string(&resume.block_md5s)
                .map_err(|error| AppError::Other(error.to_string()))?;
            let form = vec![
                ("path".to_string(), remote_path.clone()),
                ("size".to_string(), request.file_size.to_string()),
                ("isdir".to_string(), "0".to_string()),
                ("autoinit".to_string(), "1".to_string()),
                ("rtype".to_string(), "3".to_string()),
                ("block_list".to_string(), block_list),
            ];
            let precreate: PrecreateResponse = self
                .post_form(&token, "precreate", &form, request.retry_limit, &cancel)
                .await?;
            if precreate.errno != 0 {
                return Err(api_error(precreate.errno, &precreate.errmsg));
            }
            if precreate.return_type == 2 {
                progress(UploadProgress {
                    uploaded_bytes: request.file_size,
                    total_bytes: request.file_size,
                    percent: 100.0,
                    upload_id: None,
                    resume_data: None,
                });
                self.log.info("baidu", "百度网盘秒传成功");
                return Ok(UploadOutcome {
                    remote_path,
                    remote_id: None,
                    upload_id: None,
                    resume_data: None,
                    instant_upload: true,
                });
            }
            resume.upload_id = precreate.uploadid;
            resume.needed_parts = precreate.block_list;
        }

        for part_index in resume.needed_parts.clone() {
            if resume.completed_parts.contains_key(&part_index) {
                continue;
            }
            let slice = self
                .upload_slice(
                    &token,
                    &request,
                    &remote_path,
                    &resume.upload_id,
                    part_index,
                    request.retry_limit,
                    &cancel,
                )
                .await?;
            resume.completed_parts.insert(part_index, slice.md5);
            let uploaded = resume
                .completed_parts
                .keys()
                .map(|index| {
                    request
                        .file_size
                        .saturating_sub(*index as u64 * SLICE_SIZE)
                        .min(SLICE_SIZE)
                })
                .sum::<u64>();
            let resume_json = serde_json::to_string(&resume)
                .map_err(|error| AppError::Other(error.to_string()))?;
            progress(UploadProgress {
                uploaded_bytes: uploaded,
                total_bytes: request.file_size,
                percent: if request.file_size == 0 {
                    100.0
                } else {
                    uploaded as f64 * 100.0 / request.file_size as f64
                },
                upload_id: Some(resume.upload_id.clone()),
                resume_data: Some(resume_json),
            });
        }

        let completed_md5s: Vec<String> = resume
            .needed_parts
            .iter()
            .filter_map(|index| resume.completed_parts.get(index).cloned())
            .collect();
        if completed_md5s.len() != resume.needed_parts.len() {
            return Err(AppError::Provider(
                "百度分片记录不完整，任务可重试".to_string(),
            ));
        }
        let form = vec![
            ("path".to_string(), remote_path.clone()),
            ("size".to_string(), request.file_size.to_string()),
            ("isdir".to_string(), "0".to_string()),
            ("rtype".to_string(), "3".to_string()),
            ("uploadid".to_string(), resume.upload_id.clone()),
            (
                "block_list".to_string(),
                serde_json::to_string(&completed_md5s)
                    .map_err(|error| AppError::Other(error.to_string()))?,
            ),
        ];
        let created: CreateResponse = self
            .post_form(&token, "create", &form, request.retry_limit, &cancel)
            .await?;
        if created.errno != 0 {
            return Err(api_error(created.errno, &created.errmsg));
        }
        progress(UploadProgress {
            uploaded_bytes: request.file_size,
            total_bytes: request.file_size,
            percent: 100.0,
            upload_id: Some(resume.upload_id.clone()),
            resume_data: None,
        });
        self.log.info("baidu", "百度网盘文件上传完成");
        Ok(UploadOutcome {
            remote_path: if created.path.is_empty() {
                remote_path
            } else {
                created.path
            },
            remote_id: (created.fs_id > 0).then(|| created.fs_id.to_string()),
            upload_id: Some(resume.upload_id),
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
        Ok(ShareResult {
            remote_path: Some(outcome.remote_path.clone()),
            notice: Some(
                "文件上传成功，当前百度个人开放 API 文档未确认支持自动创建公开分享".to_string(),
            ),
            ..ShareResult::default()
        })
    }

    async fn get_share_result(&self, share: &ShareResult) -> AppResult<ShareResult> {
        Ok(share.clone())
    }

    async fn cancel_upload(&self, _task_id: &str) -> AppResult<()> {
        Ok(())
    }
}

fn api_error(errno: i64, message: &str) -> AppError {
    match errno {
        -6 => AppError::Authentication("百度 Token 已失效或权限不足".to_string()),
        -7 => AppError::Provider("百度网盘拒绝了文件名".to_string()),
        -9 => AppError::Provider("百度网盘目标目录不存在".to_string()),
        -10 => AppError::Provider("百度网盘空间不足".to_string()),
        31034 => AppError::Provider("百度接口请求过于频繁，可稍后重试".to_string()),
        _ => AppError::Provider(format!(
            "百度接口 errno={errno}{}",
            if message.is_empty() {
                String::new()
            } else {
                format!("：{message}")
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::api_error;

    #[test]
    fn rate_limit_is_human_readable() {
        assert!(api_error(31034, "").to_string().contains("过于频繁"));
    }
}
