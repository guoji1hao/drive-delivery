use crate::errors::{AppError, AppResult};
use chrono::Local;
use parking_lot::Mutex;
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub struct LogService {
    dir: PathBuf,
    file: Mutex<File>,
}

impl LogService {
    pub fn new(data_dir: &Path) -> AppResult<Self> {
        let dir = data_dir.join("logs");
        fs::create_dir_all(&dir)?;
        let file_path = dir.join(format!("{}.log", Local::now().format("%Y-%m-%d")));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;
        Ok(Self {
            dir,
            file: Mutex::new(file),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn info(&self, category: &str, message: &str) {
        self.write("INFO", category, message);
    }

    pub fn error(&self, category: &str, message: &str) {
        self.write("ERROR", category, message);
    }

    fn write(&self, level: &str, category: &str, message: &str) {
        let safe_message = redact(message);
        let row = json!({
            "time": Local::now().to_rfc3339(),
            "level": level,
            "category": category,
            "message": safe_message,
        });
        let mut file = self.file.lock();
        let _ = writeln!(file, "{row}");
        let _ = file.flush();
    }
}

fn redact(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("refresh_token")
        || lower.contains("client_secret")
        || lower.contains("secret_key")
        || lower.contains("authorization")
        || lower.contains("password")
        || lower.contains("cookie")
        || lower.contains("upload_id")
        || lower.contains("uploadid")
        || lower.contains("session_uri")
        || lower.contains("session_url")
        || lower.contains("uploadurl")
        || lower.contains("device_code")
    {
        "敏感认证信息已隐藏".to_string()
    } else {
        input.chars().take(1200).collect()
    }
}

impl From<keyring::Error> for AppError {
    fn from(value: keyring::Error) -> Self {
        AppError::SecureStorage(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn sensitive_values_are_not_logged() {
        assert_eq!(redact("access_token=secret"), "敏感认证信息已隐藏");
        assert_eq!(redact("Authorization: Bearer value"), "敏感认证信息已隐藏");
        assert_eq!(redact("upload_id=resume-secret"), "敏感认证信息已隐藏");
        assert_eq!(redact("普通上传错误"), "普通上传错误");
    }
}
