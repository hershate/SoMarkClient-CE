use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

const SERVICE_NAME: &str = "com.somarkclient.ce";

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("credential storage error: {0}")]
    Storage(String),
    #[error("API Key is not available for this service profile")]
    Missing,
}

#[derive(Clone)]
pub struct CredentialStore {
    portable: bool,
    session_keys: Arc<Mutex<HashMap<String, String>>>,
}

impl CredentialStore {
    pub fn new(portable: bool) -> Self {
        Self {
            portable,
            session_keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn portable(&self) -> bool {
        self.portable
    }

    pub fn set(&self, profile_id: &str, key: &str) -> Result<(), CredentialError> {
        if self.portable {
            self.session_keys
                .lock()
                .insert(profile_id.into(), key.into());
            return Ok(());
        }
        keyring::Entry::new(SERVICE_NAME, profile_id)
            .map_err(|error| CredentialError::Storage(error.to_string()))?
            .set_password(key)
            .map_err(|error| CredentialError::Storage(error.to_string()))
    }

    pub fn get(&self, profile_id: &str) -> Result<String, CredentialError> {
        let key = if self.portable {
            self.session_keys
                .lock()
                .get(profile_id)
                .cloned()
                .ok_or(CredentialError::Missing)?
        } else {
            keyring::Entry::new(SERVICE_NAME, profile_id)
                .map_err(|error| CredentialError::Storage(error.to_string()))?
                .get_password()
                .map_err(|error| match error {
                    keyring::Error::NoEntry => CredentialError::Missing,
                    other => CredentialError::Storage(other.to_string()),
                })?
        };
        Ok(normalize_api_key(&key))
    }

    pub fn delete(&self, profile_id: &str) -> Result<(), CredentialError> {
        self.session_keys.lock().remove(profile_id);
        if self.portable {
            return Ok(());
        }
        keyring::Entry::new(SERVICE_NAME, profile_id)
            .map_err(|error| CredentialError::Storage(error.to_string()))?
            .delete_credential()
            .map_err(|error| match error {
                keyring::Error::NoEntry => CredentialError::Missing,
                other => CredentialError::Storage(other.to_string()),
            })
    }
}

pub fn normalize_api_key(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '\u{feff}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}'
            )
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}
