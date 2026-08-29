use crate::errors::{AppError, AppResult};

const SERVICE_NAME: &str = "com.dualdrive.shipper";

#[derive(Default)]
pub struct SecretStore;

impl SecretStore {
    pub fn get(&self, key: &str) -> AppResult<Option<String>> {
        let entry = keyring::Entry::new(SERVICE_NAME, key)
            .map_err(|error| AppError::SecureStorage(error.to_string()))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::SecureStorage(error.to_string())),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> AppResult<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, key)
            .map_err(|error| AppError::SecureStorage(error.to_string()))?;
        entry
            .set_password(value)
            .map_err(|error| AppError::SecureStorage(error.to_string()))
    }

    pub fn delete(&self, key: &str) -> AppResult<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, key)
            .map_err(|error| AppError::SecureStorage(error.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::SecureStorage(error.to_string())),
        }
    }
}
