use crate::errors::{AppError, AppResult};

pub fn copy_text(text: &str) -> AppResult<()> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| AppError::Other(format!("无法访问系统剪贴板：{error}")))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|error| AppError::Other(format!("复制失败：{error}")))
}
