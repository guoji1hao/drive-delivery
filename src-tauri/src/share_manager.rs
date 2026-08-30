use crate::models::{AppSettings, HistoryRecord, ShareResult, UploadTask};

pub fn format_task(task: &UploadTask, settings: &AppSettings) -> String {
    apply_template(&settings.copy_template, &task.file_name, &task.baidu.share)
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
    apply_template(&settings.copy_template, &record.file_name, &baidu)
}

fn apply_template(template: &str, filename: &str, baidu: &ShareResult) -> String {
    template
        .replace("{filename}", filename)
        .replace(
            "{baidu_url}",
            baidu
                .url
                .as_deref()
                .or(baidu.notice.as_deref())
                .unwrap_or("暂未生成"),
        )
        .replace(
            "{baidu_code_line}",
            &baidu
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

#[cfg(test)]
mod tests {
    use super::apply_template;
    use crate::models::ShareResult;

    #[test]
    fn empty_codes_do_not_leave_code_lines() {
        let output = apply_template(
            "文件：{filename}\n百度：{baidu_url}\n{baidu_code_line}",
            "a.zip",
            &ShareResult {
                url: Some("https://pan.baidu.com/s/demo".into()),
                ..ShareResult::default()
            },
        );
        assert!(!output.contains("百度：\n提取码"));
        assert!(!output.contains("提取码"));
    }
}
