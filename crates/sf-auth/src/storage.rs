//! Token storage for persisting credentials.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorKind, Result};
use crate::oauth::TokenResponse;

/// Trait for token storage implementations.
pub trait TokenStorage: Send + Sync {
    /// Save a token.
    fn save(&self, key: &str, token: &TokenResponse) -> Result<()>;

    /// Load a token.
    fn load(&self, key: &str) -> Result<Option<TokenResponse>>;

    /// Delete a token.
    fn delete(&self, key: &str) -> Result<()>;

    /// Check if a token exists.
    fn exists(&self, key: &str) -> Result<bool>;

    /// List all stored token keys.
    fn list(&self) -> Result<Vec<String>>;
}

/// File-based token storage.
#[derive(Debug, Clone)]
pub struct FileTokenStorage {
    base_path: PathBuf,
}

impl FileTokenStorage {
    /// Create a new file token storage with the default path.
    ///
    /// Default path: `~/.sf-api/tokens/`
    pub fn new() -> Result<Self> {
        let base_path = default_token_dir()?;
        Ok(Self { base_path })
    }

    /// Create a new file token storage with a custom path.
    pub fn with_path(path: impl AsRef<Path>) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }

    /// Get the token file path for a key.
    fn token_path(&self, key: &str) -> PathBuf {
        // Sanitize the key to create a safe filename
        let safe_key = key
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();

        self.base_path.join(format!("{}.json", safe_key))
    }

    /// Ensure the base directory exists.
    fn ensure_dir(&self) -> Result<()> {
        if !self.base_path.exists() {
            std::fs::create_dir_all(&self.base_path)?;
        }
        Ok(())
    }
}

impl Default for FileTokenStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create default token storage")
    }
}

impl TokenStorage for FileTokenStorage {
    fn save(&self, key: &str, token: &TokenResponse) -> Result<()> {
        self.ensure_dir()?;

        let path = self.token_path(key);
        let stored = StoredToken {
            token: token.clone(),
            stored_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string_pretty(&stored)?;
        std::fs::write(&path, json)?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<TokenResponse>> {
        let path = self.token_path(key);

        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path)?;
        let stored: StoredToken = serde_json::from_str(&json)?;

        Ok(Some(stored.token))
    }

    fn delete(&self, key: &str) -> Result<()> {
        let path = self.token_path(key);

        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.token_path(key).exists())
    }

    fn list(&self) -> Result<Vec<String>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    keys.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(keys)
    }
}

/// Token with storage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredToken {
    token: TokenResponse,
    stored_at: chrono::DateTime<chrono::Utc>,
}

/// Get the default token storage directory.
pub fn default_token_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| Error::new(ErrorKind::Config("Could not find home directory".to_string())))?;

    Ok(home.join(".sf-api").join("tokens"))
}

/// Get the default token storage path.
pub fn default_token_path(key: &str) -> Result<PathBuf> {
    let dir = default_token_dir()?;
    Ok(dir.join(format!("{}.json", key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_token() -> TokenResponse {
        TokenResponse {
            access_token: "test_access".to_string(),
            refresh_token: Some("test_refresh".to_string()),
            instance_url: "https://test.salesforce.com".to_string(),
            id: None,
            token_type: Some("Bearer".to_string()),
            scope: None,
            signature: None,
            issued_at: None,
        }
    }

    #[test]
    fn test_file_storage_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileTokenStorage::with_path(temp_dir.path());

        let token = test_token();
        storage.save("test_org", &token).unwrap();

        let loaded = storage.load("test_org").unwrap().unwrap();
        assert_eq!(loaded.access_token, "test_access");
        assert_eq!(loaded.refresh_token, Some("test_refresh".to_string()));
    }

    #[test]
    fn test_file_storage_exists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileTokenStorage::with_path(temp_dir.path());

        assert!(!storage.exists("missing").unwrap());

        storage.save("exists", &test_token()).unwrap();
        assert!(storage.exists("exists").unwrap());
    }

    #[test]
    fn test_file_storage_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileTokenStorage::with_path(temp_dir.path());

        storage.save("to_delete", &test_token()).unwrap();
        assert!(storage.exists("to_delete").unwrap());

        storage.delete("to_delete").unwrap();
        assert!(!storage.exists("to_delete").unwrap());
    }

    #[test]
    fn test_file_storage_list() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileTokenStorage::with_path(temp_dir.path());

        storage.save("org1", &test_token()).unwrap();
        storage.save("org2", &test_token()).unwrap();

        let keys = storage.list().unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"org1".to_string()));
        assert!(keys.contains(&"org2".to_string()));
    }

    #[test]
    fn test_key_sanitization() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileTokenStorage::with_path(temp_dir.path());

        // Keys with special characters should be sanitized
        storage.save("user@example.com", &test_token()).unwrap();

        let path = storage.token_path("user@example.com");
        assert!(path.file_name().unwrap().to_str().unwrap().contains("user_example_com"));
    }
}
