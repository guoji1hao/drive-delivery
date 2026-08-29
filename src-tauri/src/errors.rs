use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("配置错误：{0}")]
    Configuration(String),
    #[error("认证失败：{0}")]
    Authentication(String),
    #[error("文件处理失败：{0}")]
    File(String),
    #[error("网盘接口失败：{0}")]
    Provider(String),
    #[error("任务不存在：{0}")]
    NotFound(String),
    #[error("任务已暂停")]
    Paused,
    #[error("本地数据库错误：{0}")]
    Database(String),
    #[error("系统安全存储错误：{0}")]
    SecureStorage(String),
    #[error("系统能力不可用：{0}")]
    Unavailable(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::File(value.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            Self::Provider("网络请求超时，可稍后重试".to_string())
        } else {
            Self::Provider(value.to_string())
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
