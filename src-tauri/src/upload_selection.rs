use crate::errors::{AppError, AppResult};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct SelectedEntry {
    pub path: PathBuf,
    pub relative_dir: String,
    pub is_directory: bool,
}

pub fn relative_directory(path: &Path) -> AppResult<String> {
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::Normal(value) => parts.push(
                value
                    .to_str()
                    .ok_or_else(|| AppError::File("目录名不是有效 UTF-8".into()))?,
            ),
            _ => return Err(AppError::File("文件夹相对路径无效".into())),
        }
    }
    Ok(if parts.is_empty() {
        String::new()
    } else {
        format!("{}/", parts.join("/"))
    })
}

pub fn expand_selection(paths: Vec<PathBuf>) -> AppResult<Vec<SelectedEntry>> {
    let mut entries = BTreeMap::new();
    let mut roots = paths
        .into_iter()
        .map(|p| p.canonicalize())
        .collect::<Result<Vec<_>, _>>()?;
    roots.sort_by_key(|p| p.components().count());
    let mut selected: Vec<PathBuf> = Vec::new();
    for root in roots {
        if selected.iter().any(|p| root.starts_with(p)) {
            continue;
        }
        selected.push(root.clone());
        if root.is_file() {
            entries.insert(
                root.clone(),
                SelectedEntry {
                    path: root,
                    relative_dir: String::new(),
                    is_directory: false,
                },
            );
        } else if root.is_dir() {
            let parent = root.parent().ok_or_else(|| {
                AppError::File("请选择磁盘里的文件夹，不要直接选择整个磁盘".into())
            })?;
            for entry in WalkDir::new(&root).follow_links(false) {
                let entry = entry.map_err(|e| AppError::File(e.to_string()))?;
                let path = entry.path();
                if entry.file_type().is_symlink() {
                    continue;
                }
                let is_directory = entry.file_type().is_dir();
                if !is_directory && !entry.file_type().is_file() {
                    continue;
                }
                // Nonempty directories are created by their files; explicit jobs preserve empty leaves.
                if is_directory && std::fs::read_dir(path)?.next().is_some() {
                    continue;
                }
                let relative = path
                    .strip_prefix(parent)
                    .map_err(|e| AppError::File(e.to_string()))?;
                let relative = if is_directory {
                    relative
                } else {
                    relative.parent().unwrap_or(Path::new(""))
                };
                entries.insert(
                    path.to_path_buf(),
                    SelectedEntry {
                        path: path.to_path_buf(),
                        relative_dir: relative_directory(relative)?,
                        is_directory,
                    },
                );
            }
        }
    }
    Ok(entries.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_root_nested_names_empty_folders_and_deduplicates_overlapping_selection() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("客户资料");
        std::fs::create_dir_all(root.join("甲")).unwrap();
        std::fs::create_dir_all(root.join("乙")).unwrap();
        std::fs::create_dir_all(root.join("空目录")).unwrap();
        std::fs::write(root.join("甲/a.txt"), "same").unwrap();
        std::fs::write(root.join("乙/a.txt"), "same").unwrap();
        let entries =
            expand_selection(vec![root.join("甲/a.txt"), root.clone(), root.join("甲")]).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|e| e.relative_dir == "客户资料/甲/"));
        assert!(entries.iter().any(|e| e.relative_dir == "客户资料/乙/"));
        assert!(
            entries
                .iter()
                .any(|e| e.relative_dir == "客户资料/空目录/" && e.is_directory)
        );
        assert!(relative_directory(Path::new("../escape")).is_err());
    }
}
