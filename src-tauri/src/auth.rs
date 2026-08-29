use crate::{
    database::Database,
    errors::{AppError, AppResult},
    logging::LogService,
    models::{AuthChallenge, AuthPollResult, AuthStatus, ProviderCredentialsInput},
    secret_store::SecretStore,
};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::{env, sync::Arc, time::Duration};

const OAUTH_DEVICE_CODE_URL: &str = "https://openapi.baidu.com/oauth/2.0/device/code";
const OAUTH_TOKEN_URL: &str = "https://openapi.baidu.com/oauth/2.0/token";
const KEY_APP_KEY: &str = "baidu_app_key";
const KEY_SECRET_KEY: &str = "baidu_secret_key";
const KEY_ACCESS_TOKEN: &str = "baidu_access_token";
const KEY_REFRESH_TOKEN: &str = "baidu_refresh_token";
const META_EXPIRES_AT: &str = "baidu_token_expires_at";

pub struct AuthManager {
    database: Arc<Database>,
    secrets: SecretStore,
    client: Client,
    log: Arc<LogService>,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_url: String,
    #[serde(default)]
    qrcode_url: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
}

impl AuthManager {
    pub fn new(database: Arc<Database>, log: Arc<LogService>) -> AppResult<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(45))
            .user_agent("dual-drive-assistant/0.1.0")
            .build()?;
        Ok(Self {
            database,
            secrets: SecretStore,
            client,
            log,
        })
    }

    pub fn save_baidu_credentials(&self, input: ProviderCredentialsInput) -> AppResult<()> {
        if input.app_key.trim().is_empty() || input.secret_key.trim().is_empty() {
            return Err(AppError::Configuration(
                "百度 App Key 和 Secret Key 不能为空".to_string(),
            ));
        }
        self.secrets.set(KEY_APP_KEY, input.app_key.trim())?;
        self.secrets.set(KEY_SECRET_KEY, input.secret_key.trim())?;
        self.log
            .info("auth", "百度开放平台应用凭据已保存到系统安全存储");
        Ok(())
    }

    pub fn has_baidu_credentials(&self) -> bool {
        self.baidu_credentials().is_ok()
    }

    fn baidu_credentials(&self) -> AppResult<(String, String)> {
        let app_key = self
            .secrets
            .get(KEY_APP_KEY)?
            .or_else(|| env::var("BAIDU_APP_KEY").ok())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::Configuration("请先填写百度开放平台 App Key".to_string()))?;
        let secret_key = self
            .secrets
            .get(KEY_SECRET_KEY)?
            .or_else(|| env::var("BAIDU_SECRET_KEY").ok())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                AppError::Configuration("请先填写百度开放平台 Secret Key".to_string())
            })?;
        Ok((app_key, secret_key))
    }

    pub async fn start_baidu_auth(&self) -> AppResult<AuthChallenge> {
        let (app_key, _) = self.baidu_credentials()?;
        let response = self
            .client
            .get(OAUTH_DEVICE_CODE_URL)
            .query(&[
                ("response_type", "device_code"),
                ("client_id", app_key.as_str()),
                ("scope", "basic,netdisk"),
            ])
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            self.log
                .error("auth", &format!("百度设备码请求失败 HTTP {status}"));
            return Err(AppError::Authentication(format!(
                "百度设备码请求失败，HTTP {status}"
            )));
        }
        let value: DeviceCodeResponse = serde_json::from_str(&body)
            .map_err(|error| AppError::Authentication(format!("设备码响应无法解析：{error}")))?;
        self.log.info("auth", "已创建百度设备码授权请求");
        Ok(AuthChallenge {
            provider: "baidu".to_string(),
            verification_url: if value.verification_url.is_empty() {
                value.qrcode_url
            } else {
                value.verification_url
            },
            user_code: Some(value.user_code),
            device_code: value.device_code,
            expires_in: value.expires_in,
            interval: value.interval.max(3),
        })
    }

    pub async fn poll_baidu_auth(&self, device_code: &str) -> AppResult<AuthPollResult> {
        let (app_key, secret_key) = self.baidu_credentials()?;
        let response = self
            .client
            .get(OAUTH_TOKEN_URL)
            .query(&[
                ("grant_type", "device_token"),
                ("code", device_code),
                ("client_id", app_key.as_str()),
                ("client_secret", secret_key.as_str()),
            ])
            .send()
            .await?;
        let body = response.text().await?;
        if let Ok(token) = serde_json::from_str::<TokenResponse>(&body) {
            let expires_at = self.store_token(token)?;
            self.log.info("auth", "百度网盘 OAuth 授权成功");
            return Ok(AuthPollResult {
                pending: false,
                authenticated: true,
                message: "百度网盘授权成功".to_string(),
                expires_at: Some(expires_at),
            });
        }
        let error: OAuthErrorResponse = serde_json::from_str(&body).unwrap_or(OAuthErrorResponse {
            error: None,
            error_description: None,
        });
        let code = error.error.unwrap_or_default();
        if code == "authorization_pending" || code == "slow_down" {
            return Ok(AuthPollResult {
                pending: true,
                authenticated: false,
                message: "等待用户在百度授权页确认".to_string(),
                expires_at: None,
            });
        }
        let message = error
            .error_description
            .unwrap_or_else(|| "百度授权失败或授权码已过期".to_string());
        self.log.error("auth", &message);
        Err(AppError::Authentication(message))
    }

    pub fn baidu_status(&self) -> AuthStatus {
        let expires_at = self
            .database
            .get_metadata(META_EXPIRES_AT)
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok());
        let authenticated = self.secrets.get(KEY_REFRESH_TOKEN).ok().flatten().is_some();
        AuthStatus {
            provider: "baidu".to_string(),
            authenticated,
            available: self.has_baidu_credentials(),
            expires_at,
            message: if authenticated {
                "已授权，Token 将在过期前自动刷新".to_string()
            } else if self.has_baidu_credentials() {
                "已配置开放平台应用，等待用户授权".to_string()
            } else {
                "请先配置百度开放平台 App Key 与 Secret Key".to_string()
            },
        }
    }

    pub async fn valid_baidu_access_token(&self) -> AppResult<String> {
        let now = Utc::now().timestamp();
        let expires_at = self
            .database
            .get_metadata(META_EXPIRES_AT)?
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_default();
        if expires_at > now + 120
            && let Some(access_token) = self.secrets.get(KEY_ACCESS_TOKEN)?
        {
            return Ok(access_token);
        }
        self.refresh_baidu_token().await
    }

    pub async fn refresh_baidu_token(&self) -> AppResult<String> {
        let refresh_token = self
            .secrets
            .get(KEY_REFRESH_TOKEN)?
            .ok_or_else(|| AppError::Authentication("百度网盘尚未授权".to_string()))?;
        let (app_key, secret_key) = self.baidu_credentials()?;
        let response = self
            .client
            .get(OAUTH_TOKEN_URL)
            .query(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
                ("client_id", app_key.as_str()),
                ("client_secret", secret_key.as_str()),
            ])
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        let token: TokenResponse = serde_json::from_str(&body).map_err(|_| {
            self.log
                .error("auth", &format!("百度 Token Refresh 失败 HTTP {status}"));
            AppError::Authentication("百度 Token 刷新失败，请重新授权".to_string())
        })?;
        let access_token = token.access_token.clone();
        self.store_token(token)?;
        self.log.info("auth", "百度 Access Token 已自动刷新");
        Ok(access_token)
    }

    pub fn clear_baidu_auth(&self) -> AppResult<()> {
        self.secrets.delete(KEY_ACCESS_TOKEN)?;
        self.secrets.delete(KEY_REFRESH_TOKEN)?;
        self.database.set_metadata(META_EXPIRES_AT, "0")?;
        self.log.info("auth", "已清除本机百度授权信息");
        Ok(())
    }

    fn store_token(&self, token: TokenResponse) -> AppResult<i64> {
        self.secrets.set(KEY_ACCESS_TOKEN, &token.access_token)?;
        self.secrets.set(KEY_REFRESH_TOKEN, &token.refresh_token)?;
        let expires_at = Utc::now().timestamp() + token.expires_in.max(60);
        self.database
            .set_metadata(META_EXPIRES_AT, &expires_at.to_string())?;
        Ok(expires_at)
    }
}
