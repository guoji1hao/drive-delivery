use crate::{
    errors::{AppError, AppResult},
    models::{AppSettings, HistoryRecord, ShareResult, UploadTask},
};

pub fn format_task(task: &UploadTask, settings: &AppSettings, language: &str) -> String {
    format_all(
        settings,
        &task.file_name,
        &task.baidu.share,
        &task.google.share,
        &task.onedrive.share,
        language,
    )
}

fn format_all(
    settings: &AppSettings,
    filename: &str,
    baidu: &ShareResult,
    google: &ShareResult,
    onedrive: &ShareResult,
    language: &str,
) -> String {
    if settings.copy_template != AppSettings::default().copy_template {
        return apply_template(&settings.copy_template, filename, baidu, google, onedrive);
    }
    let english = language == "en";
    let mut blocks = vec![format!(
        "{}{}",
        if english { "File: " } else { "文件：" },
        filename
    )];
    for (name, share) in [
        (
            if english {
                "Baidu Drive"
            } else {
                "百度网盘"
            },
            baidu,
        ),
        ("Google Drive", google),
        ("OneDrive", onedrive),
    ] {
        if share.url.is_some() || share.notice.is_some() {
            blocks.push(format!("{name}:\n{}", share_body(share, english)));
        }
    }
    blocks.join("\n")
}

fn share_body(share: &ShareResult, english: bool) -> String {
    let value = share
        .url
        .as_deref()
        .or(share.notice.as_deref())
        .unwrap_or(if english {
            "Not available yet"
        } else {
            "暂未生成"
        });
    let mut body = if english
        && share.url.is_none()
        && value == "文件上传成功，当前百度个人开放 API 文档未确认支持自动创建公开分享"
    {
        "Uploaded. Automatic public sharing is not confirmed for Baidu's personal open API."
            .to_string()
    } else {
        value.to_string()
    };
    if let Some(code) = share.code.as_deref().filter(|code| !code.is_empty()) {
        body.push_str(&format!(
            "\n{}{}",
            if english {
                "Access code: "
            } else {
                "提取码："
            },
            code
        ));
    }
    body
}

pub fn format_task_for(
    task: &UploadTask,
    settings: &AppSettings,
    provider: Option<&str>,
    language: &str,
) -> AppResult<String> {
    match provider {
        None | Some("all") => Ok(format_task(task, settings, language)),
        Some("baidu") => Ok(format_platform(
            "百度网盘",
            &task.file_name,
            &task.baidu.share,
            language,
        )),
        Some("onedrive") => Ok(format_platform(
            "OneDrive",
            &task.file_name,
            &task.onedrive.share,
            language,
        )),
        Some("google") => Ok(format_platform(
            "Google Drive",
            &task.file_name,
            &task.google.share,
            language,
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
    language: &str,
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
        None | Some("all") => Ok(format_all(
            settings,
            &record.file_name,
            &baidu,
            &google,
            &onedrive,
            language,
        )),
        Some("baidu") => Ok(format_platform(
            "百度网盘",
            &record.file_name,
            &baidu,
            language,
        )),
        Some("onedrive") => Ok(format_platform(
            "OneDrive",
            &record.file_name,
            &onedrive,
            language,
        )),
        Some("google") => Ok(format_platform(
            "Google Drive",
            &record.file_name,
            &google,
            language,
        )),
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

fn format_platform(name: &str, filename: &str, share: &ShareResult, language: &str) -> String {
    let english = language == "en";
    let name = if english && name == "百度网盘" {
        "Baidu Drive"
    } else {
        name
    };
    format!(
        "{}{}\n{name}:\n{}",
        if english { "File: " } else { "文件：" },
        filename,
        share_body(share, english)
    )
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
    fn default_copy_uses_language_and_only_available_drives() {
        let empty = ShareResult::default();
        let google = ShareResult {
            url: Some("https://example.test/file".into()),
            code: Some("1234".into()),
            ..ShareResult::default()
        };
        let mut settings = crate::models::AppSettings::default();
        let output = super::format_all(&settings, "中文.zip", &empty, &google, &empty, "en");
        assert_eq!(
            output,
            "File: 中文.zip\nGoogle Drive:\nhttps://example.test/file\nAccess code: 1234"
        );
        settings.copy_template = "客户文件：{filename}\n{google_url}".into();
        assert_eq!(
            super::format_all(&settings, "中文.zip", &empty, &google, &empty, "en"),
            "客户文件：中文.zip\nhttps://example.test/file"
        );
    }

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
