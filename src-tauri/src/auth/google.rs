use super::{AuthManager, GooglePendingAuth};
use crate::{
    errors::{AppError, AppResult},
    models::{AuthChallenge, AuthPollResult, AuthStatus, GoogleCredentialsInput},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{env, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};
use url::Url;
use uuid::Uuid;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DRIVE_FILE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const KEY_CLIENT_ID: &str = "google_client_id";
const KEY_CLIENT_SECRET: &str = "google_client_secret";
const KEY_ACCESS_TOKEN: &str = "google_access_token";
const KEY_REFRESH_TOKEN: &str = "google_refresh_token";
const META_EXPIRES_AT: &str = "google_token_expires_at";

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct GoogleOAuthError {
    error: Option<String>,
    error_description: Option<String>,
}

impl AuthManager {
    pub fn save_google_credentials(&self, input: GoogleCredentialsInput) -> AppResult<()> {
        let client_id = input.client_id.trim();
        if client_id.is_empty() {
            return Err(AppError::Configuration(
                "Google OAuth Client ID 不能为空".to_string(),
            ));
        }
        let client_changed = self
            .secrets
            .get(KEY_CLIENT_ID)?
            .is_some_and(|existing| existing.trim() != client_id);
        self.secrets.set(KEY_CLIENT_ID, client_id)?;
        match input.client_secret.as_deref().map(str::trim) {
            Some(secret) if !secret.is_empty() => {
                self.secrets.set(KEY_CLIENT_SECRET, secret)?;
            }
            _ => self.secrets.delete(KEY_CLIENT_SECRET)?,
        }
        if client_changed {
            self.secrets.delete(KEY_ACCESS_TOKEN)?;
            self.secrets.delete(KEY_REFRESH_TOKEN)?;
            self.database.set_metadata(META_EXPIRES_AT, "0")?;
        }
        self.log
            .info("auth", "Google OAuth 应用凭据已保存到系统安全存储");
        Ok(())
    }

    pub fn has_google_credentials(&self) -> bool {
        self.google_credentials().is_ok()
    }

    pub(super) fn google_credentials(&self) -> AppResult<(String, Option<String>)> {
        let client_id = self
            .secrets
            .get(KEY_CLIENT_ID)?
            .or_else(|| env::var("GOOGLE_CLIENT_ID").ok())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                AppError::Configuration("请先填写 Google OAuth Client ID".to_string())
            })?;
        let client_secret = self
            .secrets
            .get(KEY_CLIENT_SECRET)?
            .or_else(|| env::var("GOOGLE_CLIENT_SECRET").ok())
            .filter(|value| !value.trim().is_empty());
        Ok((client_id, client_secret))
    }

    pub async fn start_google_auth(self: &Arc<Self>) -> AppResult<AuthChallenge> {
        let (client_id, _) = self.google_credentials()?;
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|error| {
            AppError::Authentication(format!("无法启动 Google 本机授权回调：{error}"))
        })?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}");
        let state = Uuid::new_v4().simple().to_string();
        let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let expires_in = 600_u64;

        let mut authorization_url =
            Url::parse(AUTH_URL).map_err(|error| AppError::Authentication(error.to_string()))?;
        authorization_url
            .query_pairs_mut()
            .append_pair("client_id", &client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", DRIVE_FILE_SCOPE)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        {
            let now = Utc::now().timestamp();
            let mut pending = self.google_pending.lock().await;
            pending.retain(|_, value| value.expires_at > now);
            pending.insert(
                state.clone(),
                GooglePendingAuth {
                    expires_at: now + expires_in as i64,
                    result: None,
                    error: None,
                },
            );
        }
        let manager = Arc::clone(self);
        let auth_id = state.clone();
        tokio::spawn(async move {
            let result = manager
                .handle_google_callback(listener, &auth_id, &verifier, &redirect_uri)
                .await;
            if let Err(error) = result {
                manager.log.error("auth", &error.to_string());
                if let Some(pending) = manager.google_pending.lock().await.get_mut(&auth_id) {
                    pending.error = Some(error.to_string());
                }
            }
        });
        self.log.info("auth", "已创建 Google 桌面 OAuth 授权请求");

        Ok(AuthChallenge {
            provider: "google".to_string(),
            verification_url: authorization_url.to_string(),
            user_code: None,
            device_code: state,
            expires_in,
            interval: 2,
        })
    }

    async fn handle_google_callback(
        &self,
        listener: TcpListener,
        expected_state: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> AppResult<()> {
        let (mut stream, _) = timeout(Duration::from_secs(600), listener.accept())
            .await
            .map_err(|_| AppError::Authentication("Google 授权已超时".to_string()))?
            .map_err(|error| AppError::Authentication(format!("Google 回调接收失败：{error}")))?;
        let callback = read_callback(&mut stream).await;
        let result = match callback {
            Ok((code, state)) if state == expected_state => {
                self.exchange_google_code(&code, verifier, redirect_uri)
                    .await
            }
            Ok((_, _)) => Err(AppError::Authentication(
                "Google OAuth state 校验失败，请重新授权".to_string(),
            )),
            Err(error) => Err(error),
        };
        let success = result.is_ok();
        let page = if success {
            "<!doctype html><meta charset=\"utf-8\"><title>授权成功</title><h1>Google Drive 授权成功</h1><p>可以关闭此页面并返回双盘发货助手。</p>"
        } else {
            "<!doctype html><meta charset=\"utf-8\"><title>授权失败</title><h1>Google Drive 授权失败</h1><p>请返回双盘发货助手查看错误并重新授权。</p>"
        };
        let response = format!(
            "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
            if success { "200 OK" } else { "400 Bad Request" },
            page.len(),
            page
        );
        let _ = stream.write_all(response.as_bytes()).await;
        result?;

        if let Some(pending) = self.google_pending.lock().await.get_mut(expected_state) {
            pending.result = Some(AuthPollResult {
                pending: false,
                authenticated: true,
                message: "Google Drive 授权成功".to_string(),
                expires_at: self.google_expiry(),
            });
        }
        self.log.info("auth", "Google Drive OAuth 授权成功");
        Ok(())
    }

    async fn exchange_google_code(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> AppResult<()> {
        let (client_id, client_secret) = self.google_credentials()?;
        let mut form = vec![
            ("client_id", client_id.as_str()),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];
        if let Some(secret) = client_secret.as_deref() {
            form.push(("client_secret", secret));
        }
        let response = self.client.post(TOKEN_URL).form(&form).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(google_oauth_error(status.as_u16(), &body));
        }
        let token: GoogleTokenResponse = serde_json::from_str(&body).map_err(|error| {
            AppError::Authentication(format!("Google Token 响应无法解析：{error}"))
        })?;
        self.store_google_token(token)?;
        Ok(())
    }

    pub async fn poll_google_auth(&self, auth_id: &str) -> AppResult<AuthPollResult> {
        let mut pending = self.google_pending.lock().await;
        let entry = pending
            .get_mut(auth_id)
            .ok_or_else(|| AppError::Authentication("Google 授权请求不存在或已结束".to_string()))?;
        if entry.expires_at <= Utc::now().timestamp() {
            pending.remove(auth_id);
            return Err(AppError::Authentication("Google 授权已超时".to_string()));
        }
        if let Some(error) = entry.error.take() {
            pending.remove(auth_id);
            return Err(AppError::Authentication(error));
        }
        if let Some(result) = entry.result.take() {
            pending.remove(auth_id);
            return Ok(result);
        }
        Ok(AuthPollResult {
            pending: true,
            authenticated: false,
            message: "等待用户在系统浏览器完成 Google 授权".to_string(),
            expires_at: None,
        })
    }

    pub fn google_status(&self) -> AuthStatus {
        let authenticated = self.secrets.get(KEY_REFRESH_TOKEN).ok().flatten().is_some();
        let available = self.has_google_credentials();
        AuthStatus {
            provider: "google".to_string(),
            authenticated,
            available,
            expires_at: self.google_expiry(),
            message: if authenticated {
                "已授权，Token 将在过期前自动刷新".to_string()
            } else if available {
                "已配置桌面 OAuth 应用，等待用户授权".to_string()
            } else {
                "请先配置 Google 桌面 OAuth Client ID".to_string()
            },
        }
    }

    fn google_expiry(&self) -> Option<i64> {
        self.database
            .get_metadata(META_EXPIRES_AT)
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
    }

    pub async fn valid_google_access_token(&self) -> AppResult<String> {
        let now = Utc::now().timestamp();
        if self.google_expiry().unwrap_or_default() > now + 120
            && let Some(access_token) = self.secrets.get(KEY_ACCESS_TOKEN)?
        {
            return Ok(access_token);
        }
        self.refresh_google_token().await
    }

    pub async fn refresh_google_token(&self) -> AppResult<String> {
        let refresh_token = self
            .secrets
            .get(KEY_REFRESH_TOKEN)?
            .ok_or_else(|| AppError::Authentication("Google Drive 尚未授权".to_string()))?;
        let (client_id, client_secret) = self.google_credentials()?;
        let mut form = vec![
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];
        if let Some(secret) = client_secret.as_deref() {
            form.push(("client_secret", secret));
        }
        let response = self.client.post(TOKEN_URL).form(&form).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            self.log
                .error("auth", &format!("Google Token Refresh 失败 HTTP {status}"));
            return Err(AppError::Authentication(
                "Google Token 刷新失败，请重新授权".to_string(),
            ));
        }
        let token: GoogleTokenResponse = serde_json::from_str(&body).map_err(|_| {
            AppError::Authentication("Google Token 刷新响应无法解析，请重新授权".to_string())
        })?;
        let access_token = token.access_token.clone();
        self.store_google_token(token)?;
        self.log.info("auth", "Google Access Token 已自动刷新");
        Ok(access_token)
    }

    pub fn clear_google_auth(&self) -> AppResult<()> {
        self.secrets.delete(KEY_ACCESS_TOKEN)?;
        self.secrets.delete(KEY_REFRESH_TOKEN)?;
        self.database.set_metadata(META_EXPIRES_AT, "0")?;
        self.log.info("auth", "已清除本机 Google 授权信息");
        Ok(())
    }

    fn store_google_token(&self, token: GoogleTokenResponse) -> AppResult<i64> {
        self.secrets.set(KEY_ACCESS_TOKEN, &token.access_token)?;
        if let Some(refresh_token) = token.refresh_token {
            self.secrets.set(KEY_REFRESH_TOKEN, &refresh_token)?;
        } else if self.secrets.get(KEY_REFRESH_TOKEN)?.is_none() {
            return Err(AppError::Authentication(
                "Google 未返回 Refresh Token，请撤销旧授权后重新绑定".to_string(),
            ));
        }
        let expires_at = Utc::now().timestamp() + token.expires_in.max(60);
        self.database
            .set_metadata(META_EXPIRES_AT, &expires_at.to_string())?;
        Ok(expires_at)
    }
}

async fn read_callback(stream: &mut TcpStream) -> AppResult<(String, String)> {
    let mut buffer = vec![0_u8; 8192];
    let count = timeout(Duration::from_secs(30), stream.read(&mut buffer))
        .await
        .map_err(|_| AppError::Authentication("Google 回调读取超时".to_string()))??;
    let request = String::from_utf8_lossy(&buffer[..count]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| AppError::Authentication("Google 回调格式无效".to_string()))?;
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| AppError::Authentication("Google 回调地址无效".to_string()))?;
    let mut code = None;
    let mut state = None;
    let mut oauth_error = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => oauth_error = Some(value.into_owned()),
            _ => {}
        }
    }
    if let Some(error) = oauth_error {
        return Err(AppError::Authentication(format!(
            "用户未完成 Google 授权：{error}"
        )));
    }
    Ok((
        code.ok_or_else(|| AppError::Authentication("Google 回调缺少授权码".to_string()))?,
        state.ok_or_else(|| AppError::Authentication("Google 回调缺少 state".to_string()))?,
    ))
}

fn google_oauth_error(status: u16, body: &str) -> AppError {
    let value = serde_json::from_str::<GoogleOAuthError>(body).ok();
    let detail = value
        .and_then(|value| value.error_description.or(value.error))
        .unwrap_or_else(|| format!("HTTP {status}"));
    AppError::Authentication(format!("Google OAuth 失败：{detail}"))
}

#[cfg(test)]
mod tests {
    use super::DRIVE_FILE_SCOPE;

    #[test]
    fn uses_narrow_drive_scope() {
        assert_eq!(
            DRIVE_FILE_SCOPE,
            "https://www.googleapis.com/auth/drive.file"
        );
    }
}
