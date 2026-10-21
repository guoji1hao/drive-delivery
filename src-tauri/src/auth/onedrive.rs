use super::AuthManager;
use crate::{
    errors::{AppError, AppResult},
    models::{AuthChallenge, AuthPollResult, AuthStatus},
};
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

const AUTHORITY: &str = "https://login.microsoftonline.com/common/oauth2/v2.0";
const SCOPE: &str = "https://graph.microsoft.com/Files.ReadWrite offline_access";
const CLIENT: &str = "onedrive_client_id";
const ACCESS: &str = "onedrive_access_token";
const REFRESH: &str = "onedrive_refresh_token";
const EXPIRY: &str = "onedrive_token_expires_at";

pub(super) struct OneDrivePending {
    code: String,
    client_id: String,
    expires_at: i64,
    next_poll: i64,
    interval: i64,
}

#[derive(Deserialize)]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    #[serde(default = "default_interval")]
    interval: u64,
}
fn default_interval() -> u64 {
    5
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

fn pending() -> AuthPollResult {
    AuthPollResult {
        pending: true,
        authenticated: false,
        message: "等待 Microsoft 授权确认".into(),
        expires_at: None,
    }
}

impl AuthManager {
    pub async fn save_onedrive_credentials(&self, client_id: &str) -> AppResult<()> {
        let client_id = client_id.trim();
        Uuid::parse_str(client_id).map_err(|_| {
            AppError::Configuration("Microsoft Client ID 必须是应用注册中的 UUID".into())
        })?;
        if self.onedrive_client_id().ok().as_deref() != Some(client_id) {
            self.clear_onedrive_auth().await?;
        }
        self.secrets.set(CLIENT, client_id)
    }

    fn onedrive_client_id(&self) -> AppResult<String> {
        self.secrets
            .get(CLIENT)?
            .or_else(|| std::env::var("ONEDRIVE_CLIENT_ID").ok())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| AppError::Configuration("请先填写 Microsoft 应用 Client ID".into()))
    }

    pub fn onedrive_status(&self) -> AuthStatus {
        let authenticated = self.secrets.get(REFRESH).ok().flatten().is_some();
        AuthStatus {
            provider: "onedrive".into(),
            authenticated,
            available: self.onedrive_client_id().is_ok(),
            expires_at: self
                .database
                .get_metadata(EXPIRY)
                .ok()
                .flatten()
                .and_then(|s| s.parse().ok()),
            message: if authenticated {
                "已授权，Token 将自动刷新"
            } else {
                "填写 Microsoft Client ID 后通过官方设备码绑定"
            }
            .into(),
        }
    }

    pub async fn start_onedrive_auth(&self) -> AppResult<AuthChallenge> {
        let client_id = self.onedrive_client_id()?;
        let response = self
            .client
            .post(format!("{AUTHORITY}/devicecode"))
            .form(&[("client_id", client_id.as_str()), ("scope", SCOPE)])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AppError::Authentication(format!(
                "Microsoft 设备码请求失败 HTTP {}；确认已允许公共客户端流",
                response.status()
            )));
        }
        let code: DeviceCode = response.json().await?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        let interval = code.interval.max(5) as i64;
        let mut requests = self.onedrive_pending.lock().await;
        requests.clear();
        requests.insert(
            id.clone(),
            OneDrivePending {
                code: code.device_code,
                client_id,
                expires_at: now + code.expires_in as i64,
                next_poll: now + interval,
                interval,
            },
        );
        Ok(AuthChallenge {
            provider: "onedrive".into(),
            verification_url: code.verification_uri,
            user_code: Some(code.user_code),
            device_code: id,
            expires_in: code.expires_in,
            interval: interval as u64,
        })
    }

    pub async fn poll_onedrive_auth(&self, id: &str) -> AppResult<AuthPollResult> {
        let mut requests = self.onedrive_pending.lock().await;
        let now = Utc::now().timestamp();
        let request = requests
            .get_mut(id)
            .ok_or_else(|| AppError::Authentication("Microsoft 授权请求已失效".into()))?;
        if now >= request.expires_at {
            requests.remove(id);
            return Err(AppError::Authentication(
                "Microsoft 授权已过期，请重新绑定".into(),
            ));
        }
        if now < request.next_poll {
            return Ok(pending());
        }
        request.next_poll = now + request.interval;
        let response = self
            .client
            .post(format!("{AUTHORITY}/token"))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", request.client_id.as_str()),
                ("device_code", request.code.as_str()),
            ])
            .send()
            .await?;
        if response.status().is_success() {
            let token: Token = response.json().await?;
            let expires_at = self.store_onedrive_token(token)?;
            requests.remove(id);
            return Ok(AuthPollResult {
                pending: false,
                authenticated: true,
                message: "OneDrive 授权成功".into(),
                expires_at: Some(expires_at),
            });
        }
        let body: serde_json::Value = response.json().await?;
        match body["error"].as_str() {
            Some("authorization_pending") => Ok(pending()),
            Some("slow_down") => {
                request.interval += 5;
                request.next_poll = now + request.interval;
                Ok(pending())
            }
            _ => {
                requests.remove(id);
                Err(AppError::Authentication(
                    "Microsoft 授权被拒绝或已失效，请重新绑定".into(),
                ))
            }
        }
    }

    pub async fn valid_onedrive_access_token(&self) -> AppResult<String> {
        let _guard = self.onedrive_refresh.lock().await;
        let expires = self
            .database
            .get_metadata(EXPIRY)?
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        if expires > Utc::now().timestamp() + 120
            && let Some(token) = self.secrets.get(ACCESS)?
        {
            return Ok(token);
        }
        self.refresh_onedrive_token_inner().await
    }

    pub async fn refresh_onedrive_token(&self) -> AppResult<String> {
        let _guard = self.onedrive_refresh.lock().await;
        self.refresh_onedrive_token_inner().await
    }

    async fn refresh_onedrive_token_inner(&self) -> AppResult<String> {
        let refresh = self
            .secrets
            .get(REFRESH)?
            .ok_or_else(|| AppError::Authentication("OneDrive 尚未授权".into()))?;
        let client_id = self.onedrive_client_id()?;
        let response = self
            .client
            .post(format!("{AUTHORITY}/token"))
            .timeout(Duration::from_secs(45))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id.as_str()),
                ("refresh_token", refresh.as_str()),
                ("scope", SCOPE),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AppError::Authentication(
                "Microsoft Token 刷新失败，请重新绑定".into(),
            ));
        }
        let token: Token = response.json().await?;
        let access = token.access_token.clone();
        self.store_onedrive_token(token)?;
        Ok(access)
    }

    fn store_onedrive_token(&self, token: Token) -> AppResult<i64> {
        if let Some(refresh) = token.refresh_token {
            self.secrets.set(REFRESH, &refresh)?;
        } else if self.secrets.get(REFRESH)?.is_none() {
            return Err(AppError::Authentication(
                "Microsoft 未返回 Refresh Token，请重新授权 offline_access".into(),
            ));
        }
        self.secrets.set(ACCESS, &token.access_token)?;
        let expires = Utc::now().timestamp() + token.expires_in;
        self.database.set_metadata(EXPIRY, &expires.to_string())?;
        Ok(expires)
    }

    pub async fn clear_onedrive_auth(&self) -> AppResult<()> {
        let mut requests = self.onedrive_pending.lock().await;
        let _guard = self.onedrive_refresh.lock().await;
        requests.clear();
        self.secrets.delete(ACCESS)?;
        self.secrets.delete(REFRESH)?;
        self.database.set_metadata(EXPIRY, "0")
    }
}
