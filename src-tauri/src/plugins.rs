//! Declarative plugin packages select trusted adapters bundled with this build.
use crate::{
    adapters::DriveAdapter,
    database::Database,
    errors::{AppError, AppResult},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub api_version: u32,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrivePlugin {
    pub manifest: PluginManifest,
    pub installed: bool,
    pub enabled: bool,
}

pub fn catalog() -> Vec<PluginManifest> {
    [
        include_str!("../../plugins/baidu.drive-plugin.json"),
        include_str!("../../plugins/google.drive-plugin.json"),
        include_str!("../../plugins/onedrive.drive-plugin.json"),
    ]
    .iter()
    .map(|json| serde_json::from_str(json).expect("bundled manifest"))
    .collect()
}

pub fn parse_manifest(json: &str) -> AppResult<PluginManifest> {
    if json.len() > 4096 {
        return Err(AppError::Configuration("插件文件不能超过 4 KB".into()));
    }
    let manifest: PluginManifest = serde_json::from_str(json)
        .map_err(|_| AppError::Configuration("插件文件格式不正确".into()))?;
    if !catalog().contains(&manifest) {
        return Err(AppError::Configuration(
            "此插件需要其他版本的软件，或尚未支持该网盘".into(),
        ));
    }
    Ok(manifest)
}

pub struct PluginService {
    database: Arc<Database>,
    entries: Mutex<Vec<DrivePlugin>>,
    adapters: HashMap<String, Arc<dyn DriveAdapter>>,
}

impl PluginService {
    pub fn new(database: Arc<Database>, adapters: Vec<Arc<dyn DriveAdapter>>) -> AppResult<Self> {
        let entries = match database.get_metadata("drive_plugins_v1")? {
            Some(json) => {
                let entries: Vec<DrivePlugin> = serde_json::from_str(&json).map_err(|_| {
                    AppError::Configuration("网盘插件配置损坏，请检查本机数据".into())
                })?;
                let supported = catalog();
                if entries.len() != supported.len()
                    || supported
                        .iter()
                        .any(|m| entries.iter().filter(|e| e.manifest == *m).count() != 1)
                    || entries.iter().any(|e| e.enabled && !e.installed)
                {
                    return Err(AppError::Configuration(
                        "网盘插件配置损坏，请检查本机数据".into(),
                    ));
                }
                entries
            }
            None => catalog()
                .into_iter()
                .map(|manifest| DrivePlugin {
                    manifest,
                    installed: true,
                    enabled: true,
                })
                .collect(),
        };
        Ok(Self {
            database,
            entries: Mutex::new(entries),
            adapters: adapters
                .into_iter()
                .map(|a| (a.provider().to_string(), a))
                .collect(),
        })
    }

    pub fn list(&self) -> Vec<DrivePlugin> {
        self.entries.lock().clone()
    }
    pub fn enabled(&self, id: &str) -> bool {
        self.entries
            .lock()
            .iter()
            .any(|e| e.manifest.id == id && e.installed && e.enabled)
    }
    pub fn adapter(&self, id: &str) -> AppResult<Arc<dyn DriveAdapter>> {
        self.adapters
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::Configuration("网盘适配器未安装".into()))
    }
    pub fn update(
        &self,
        id: &str,
        action: &str,
        json: Option<&str>,
    ) -> AppResult<Vec<DrivePlugin>> {
        if let Some(json) = json {
            if action != "install" || parse_manifest(json)?.id != id {
                return Err(AppError::Configuration("插件编号不匹配".into()));
            }
        }
        let mut guard = self.entries.lock();
        let mut next = guard.clone();
        let entry = next
            .iter_mut()
            .find(|e| e.manifest.id == id)
            .ok_or_else(|| AppError::Configuration("不支持的网盘插件".into()))?;
        match action {
            "install" => {
                entry.installed = true;
                entry.enabled = true;
            }
            "enable" if entry.installed => entry.enabled = true,
            "disable" if entry.installed => entry.enabled = false,
            "remove" => {
                entry.installed = false;
                entry.enabled = false;
            }
            _ => {
                return Err(AppError::Configuration(
                    "请先安装插件，或检查操作类型".into(),
                ));
            }
        }
        self.database.set_metadata(
            "drive_plugins_v1",
            &serde_json::to_string(&next)
                .map_err(|_| AppError::Configuration("无法保存插件配置".into()))?,
        )?;
        *guard = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packages_reject_code_unknown_drivers_and_versions() {
        assert!(parse_manifest(r#"{"id":"google","apiVersion":1,"version":"1.0.0"}"#).is_ok());
        for json in [
            r#"{"id":"unknown","apiVersion":1,"version":"1.0.0"}"#,
            r#"{"id":"google","apiVersion":2,"version":"1.0.0"}"#,
            r#"{"id":"google","apiVersion":1,"version":"1.0.0","script":"run"}"#,
        ] {
            assert!(parse_manifest(json).is_err());
        }
        assert!(parse_manifest(&"x".repeat(4097)).is_err());
    }
    #[test]
    fn lifecycle_persists_without_touching_other_plugins() {
        let path = std::env::temp_dir().join(format!("plugins-{}.db", uuid::Uuid::new_v4()));
        {
            let db = Arc::new(Database::open(&path).unwrap());
            let service = PluginService::new(db.clone(), vec![]).unwrap();
            assert!(service.enabled("google"));
            service.update("google", "disable", None).unwrap();
            assert!(!service.enabled("google"));
            assert!(service.enabled("baidu"));
            service.update("google", "remove", None).unwrap();
            assert!(service.update("google", "enable", None).is_err());
            let reloaded = PluginService::new(db, vec![]).unwrap();
            assert!(!reloaded.enabled("google"));
            reloaded.update("google", "install", None).unwrap();
            assert!(reloaded.enabled("google"));
        }
        let _ = std::fs::remove_file(path);
    }
}
