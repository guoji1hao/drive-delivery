use crate::{
    errors::{AppError, AppResult},
    models::{AppSettings, HistoryRecord, ShareResult, UploadTask},
};

pub fn format_task(task: &UploadTask, settings: &AppSettings) -> String {
    apply_template(
        &settings.copy_template,
        &task.file_name,
        &task.baidu.share,
        &task.google.share,
        &task.onedrive.share,
    )
}

pub fn format_task_for(
    task: &UploadTask,
    settings: &AppSettings,
    provider: Option<&str>,
) -> AppResult<String> {
    match provider {
        None | Some("all") => Ok(format_task(task, settings)),
        Some("baidu") => Ok(format_platform(
            "百度网盘",
            &task.file_name,
            &task.baidu.share,
        )),
        Some("onedrive") => Ok(format_platform(
            "OneDrive",
            &task.file_name,
            &task.onedrive.share,
        )),
        Some("google") => Ok(format_platform(
            "Google Drive",
            &task.file_name,
            &task.google.share,
        )),
        Some(value) => Err(AppError::Configuration(format!(
            "不支持的复制平台：{value}"
        ))),
    }
}

pub fn format_history_for(
    record: &HistoryRecord,
    settings: &AppSettings,
    provider: Option<&str>,
) -> AppResult<String> {
    let baidu = history_share(
        &record.baidu_url,
        &record.baidu_code,
        record.baidu_expires_at,
        &record.baidu_share_id,
        &record.baidu_path,
    );
    let google = history_share(
        &record.google_url,
        &record.google_code,
        record.google_expires_at,
        &record.google_share_id,
        &record.google_path,
    );
    let onedrive = history_share(
        &record.onedrive_url,
        &record.onedrive_code,
        record.onedrive_expires_at,
        &record.onedrive_share_id,
        &record.onedrive_path,
    );
    match provider {
        None | Some("all") => Ok(apply_template(
            &settings.copy_template,
            &record.file_name,
            &baidu,
            &google,
            &onedrive,
        )),
        Some("baidu") => Ok(format_platform("百度网盘", &record.file_name, &baidu)),
        Some("onedrive") => Ok(format_platform("OneDrive", &record.file_name, &onedrive)),
        Some("google") => Ok(format_platform("Google Drive", &record.file_name, &google)),
        Some(value) => Err(AppError::Configuration(format!(
            "不支持的复制平台：{value}"
        ))),
    }
}

fn history_share(
    url: &Option<String>,
    code: &Option<String>,
    expires_at: Option<i64>,
    share_id: &Option<String>,
    remote_path: &Option<String>,
) -> ShareResult {
    ShareResult {
        url: url.clone(),
        code: code.clone(),
        expires_at,
        share_id: share_id.clone(),
        remote_path: remote_path.clone(),
        notice: None,
    }
}

fn format_platform(name: &str, filename: &str, share: &ShareResult) -> String {
    let value = share
        .url
        .as_deref()
        .or(share.notice.as_deref())
        .unwrap_or("暂未生成");
    let mut lines = vec![
        format!("文件：{filename}"),
        format!("{name}："),
        value.to_string(),
    ];
    if let Some(code) = share.code.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("提取码：{code}"));
    }
    lines.join("\n")
}

fn apply_template(
    template: &str,
    filename: &str,
    baidu: &ShareResult,
    google: &ShareResult,
    onedrive: &ShareResult,
) -> String {
    template
        .replace("{filename}", filename)
        .replace("{baidu_url}", share_value(baidu))
        .replace("{baidu_code_line}", &code_line(baidu))
        .replace("{google_url}", share_value(google))
        .replace("{google_code_line}", &code_line(google))
        .replace("{onedrive_url}", share_value(onedrive))
        .replace("{onedrive_code_line}", &code_line(onedrive))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn share_value(share: &ShareResult) -> &str {
    share
        .url
        .as_deref()
        .or(share.notice.as_deref())
        .unwrap_or("暂未生成")
}

fn code_line(share: &ShareResult) -> String {
    share
        .code
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|code| format!("提取码：{code}"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::apply_template;
    use crate::models::ShareResult;

    #[test]
    fn includes_onedrive_in_default_template() {
        let output = apply_template(
            &crate::models::AppSettings::default().copy_template,
            "中文.zip",
            &ShareResult::default(),
            &ShareResult::default(),
            &ShareResult {
                url: Some("https://example.test/ms".into()),
                ..ShareResult::default()
            },
        );
        assert!(output.contains("OneDrive：\nhttps://example.test/ms"));
        assert!(!output.contains("{onedrive"));
    }

    #[test]
    fn empty_codes_do_not_leave_code_lines() {
        let output = apply_template(
            "文件：{filename}\n百度：{baidu_url}\n{baidu_code_line}\nGoogle：{google_url}\n{google_code_line}",
            "a.zip",
            &ShareResult {
                url: Some("https://pan.baidu.com/s/demo".into()),
                ..ShareResult::default()
            },
            &ShareResult {
                url: Some("https://drive.google.com/file/d/demo/view".into()),
                ..ShareResult::default()
            },
            &ShareResult::default(),
        );
        assert!(!output.contains("提取码"));
        assert!(output.contains("drive.google.com"));
    }
}
