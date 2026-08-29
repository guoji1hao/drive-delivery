use crate::{
    database::Database,
    errors::{AppError, AppResult},
    models::AppSettings,
};
use std::sync::Arc;

pub struct SettingsService {
    database: Arc<Database>,
}

impl SettingsService {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    pub fn get(&self) -> AppResult<AppSettings> {
        self.database.load_settings()
    }

    pub fn save(&self, mut settings: AppSettings) -> AppResult<AppSettings> {
        if !(1..=8).contains(&settings.max_concurrent_files) {
            return Err(AppError::Configuration(
                "同时处理文件数必须在 1 到 8 之间".to_string(),
            ));
        }
        if settings.retry_limit > 8 {
            return Err(AppError::Configuration(
                "自动重试次数不能超过 8".to_string(),
            ));
        }
        settings.baidu_default_dir = normalize_dir(&settings.baidu_default_dir);
        settings.quark_default_dir = normalize_dir(&settings.quark_default_dir);
        if !settings.baidu_default_dir.starts_with("/apps/") {
            return Err(AppError::Configuration(
                "百度开放平台上传目录必须位于 /apps/应用目录/ 下".to_string(),
            ));
        }
        if settings.copy_template.trim().is_empty() {
            settings.copy_template = AppSettings::default().copy_template;
        }
        self.database.save_settings(&settings)?;
        Ok(settings)
    }
}

pub fn normalize_dir(input: &str) -> String {
    let mut value = input.trim().replace('\\', "/");
    if !value.starts_with('/') {
        value.insert(0, '/');
    }
    while value.contains("//") {
        value = value.replace("//", "/");
    }
    if !value.ends_with('/') {
        value.push('/');
    }
    value
}

#[cfg(test)]
mod tests {
    use super::normalize_dir;

    #[test]
    fn drive_directories_are_normalized() {
        assert_eq!(normalize_dir("apps\\demo\\发货"), "/apps/demo/发货/");
    }
}
