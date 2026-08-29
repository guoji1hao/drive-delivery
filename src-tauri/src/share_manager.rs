use crate::models::{AppSettings, HistoryRecord, ShareResult, UploadTask};

pub fn format_task(task: &UploadTask, settings: &AppSettings, provider: Option<&str>) -> String {
    match provider {
        Some("baidu") => format_provider("百度网盘", &task.baidu.share),
        Some("quark") => format_provider("夸克网盘", &task.quark.share),
        _ => apply_template(
            &settings.copy_template,
            &task.file_name,
            &task.baidu.share,
            &task.quark.share,
        ),
    }
}

pub fn format_history(record: &HistoryRecord, settings: &AppSettings) -> String {
    let baidu = ShareResult {
        url: record.baidu_url.clone(),
        code: record.baidu_code.clone(),
        expires_at: record.baidu_expires_at,
        share_id: record.baidu_share_id.clone(),
        remote_path: record.baidu_path.clone(),
        notice: None,
    };
    let quark = ShareResult {
        url: record.quark_url.clone(),
        code: record.quark_code.clone(),
        expires_at: record.quark_expires_at,
        share_id: record.quark_share_id.clone(),
        remote_path: record.quark_path.clone(),
        notice: None,
    };
    apply_template(&settings.copy_template, &record.file_name, &baidu, &quark)
}

fn apply_template(
    template: &str,
    filename: &str,
    baidu: &ShareResult,
    quark: &ShareResult,
) -> String {
    template
        .replace("{filename}", filename)
        .replace("{baidu_url}", baidu.url.as_deref().unwrap_or("暂未生成"))
        .replace("{quark_url}", quark.url.as_deref().unwrap_or("暂未生成"))
        .replace(
            "{baidu_code_line}",
            &baidu
                .code
                .as_deref()
                .map(|code| format!("提取码：{code}"))
                .unwrap_or_default(),
        )
        .replace(
            "{quark_code_line}",
            &quark
                .code
                .as_deref()
                .map(|code| format!("提取码：{code}"))
                .unwrap_or_default(),
        )
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_provider(name: &str, share: &ShareResult) -> String {
    let mut lines = vec![name.to_string()];
    if let Some(url) = &share.url {
        lines.push(url.clone());
    } else if let Some(notice) = &share.notice {
        lines.push(notice.clone());
    } else {
        lines.push("暂未生成分享链接".to_string());
    }
    if let Some(code) = &share.code {
        lines.push(format!("提取码：{code}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::apply_template;
    use crate::models::ShareResult;

    #[test]
    fn empty_codes_do_not_leave_code_lines() {
        let output = apply_template(
            "文件：{filename}\n百度：{baidu_url}\n{baidu_code_line}\n夸克：{quark_url}\n{quark_code_line}",
            "a.zip",
            &ShareResult {
                url: Some("https://pan.baidu.com/s/demo".into()),
                ..ShareResult::default()
            },
            &ShareResult {
                url: Some("https://pan.quark.cn/s/demo".into()),
                code: Some("1234".into()),
                ..ShareResult::default()
            },
        );
        assert!(!output.contains("百度：\n提取码"));
        assert_eq!(output.matches("提取码").count(), 1);
    }
}
