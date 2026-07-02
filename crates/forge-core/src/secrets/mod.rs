use std::path::PathBuf;
use std::collections::HashMap;
use std::fs;
use serde::{Serialize, Deserialize};
use keyring::Entry;
use rand::Rng;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};

pub trait SecretProvider: Send + Sync {
    fn name(&self) -> &str;
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
    fn list(&self) -> Result<Vec<String>, String>;
}

pub trait ConfigurationProvider: Send + Sync {
    fn name(&self) -> &str;
    fn load(&self, ctx: &dyn crate::environment::RuntimeContextProvider) -> Result<HashMap<String, String>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueSource {
    CliOverride,
    SystemEnv,
    LocalOverride,
    SecretProvider(String),
    EnvFile,
    ProfileOverlay(String),
    DefaultManifest,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarMetadata {
    pub key: String,
    pub source: ValueSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEnvironment {
    pub vars: HashMap<String, String>,
    pub metadata: HashMap<String, VarMetadata>,
}

// OS Keyring Secret Provider
pub struct KeyringSecretProvider {
    pub workspace_id: String,
}

impl KeyringSecretProvider {
    pub fn new(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
        }
    }
}

impl SecretProvider for KeyringSecretProvider {
    fn name(&self) -> &str {
        "keyring"
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let entry_name = format!("{}::{}", self.workspace_id, key);
        let entry = Entry::new("forge-secrets", &entry_name)
            .map_err(|e| format!("Keyring error: {}", e))?;
        match entry.get_password() {
            Ok(pwd) => Ok(Some(pwd)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Keyring get error: {}", e)),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let entry_name = format!("{}::{}", self.workspace_id, key);
        let entry = Entry::new("forge-secrets", &entry_name)
            .map_err(|e| format!("Keyring error: {}", e))?;
        entry.set_password(value)
            .map_err(|e| format!("Keyring set error: {}", e))?;
        
        // Track key in the list of keys
        let mut keys = self.list()?;
        if !keys.contains(&key.to_string()) {
            keys.push(key.to_string());
            let list_entry_name = format!("{}::_keys", self.workspace_id);
            let list_entry = Entry::new("forge-secrets", &list_entry_name)
                .map_err(|e| format!("Keyring error: {}", e))?;
            list_entry.set_password(&keys.join(","))
                .map_err(|e| format!("Keyring list set error: {}", e))?;
        }
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let entry_name = format!("{}::{}", self.workspace_id, key);
        let entry = Entry::new("forge-secrets", &entry_name)
            .map_err(|e| format!("Keyring error: {}", e))?;
        match entry.delete_password() {
            Ok(_) => {}
            Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(format!("Keyring delete error: {}", e)),
        }

        // Untrack key
        let mut keys = self.list()?;
        if let Some(pos) = keys.iter().position(|k| k == key) {
            keys.remove(pos);
            let list_entry_name = format!("{}::_keys", self.workspace_id);
            let list_entry = Entry::new("forge-secrets", &list_entry_name)
                .map_err(|e| format!("Keyring error: {}", e))?;
            if keys.is_empty() {
                let _ = list_entry.delete_password();
            } else {
                list_entry.set_password(&keys.join(","))
                    .map_err(|e| format!("Keyring list set error: {}", e))?;
            }
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, String> {
        let entry_name = format!("{}::_keys", self.workspace_id);
        let entry = Entry::new("forge-secrets", &entry_name)
            .map_err(|e| format!("Keyring error: {}", e))?;
        match entry.get_password() {
            Ok(pwd) => {
                let keys: Vec<String> = pwd.split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                Ok(keys)
            }
            Err(keyring::Error::NoEntry) => Ok(Vec::new()),
            Err(e) => Err(format!("Keyring list error: {}", e)),
        }
    }
}

// Mock Secret Provider
pub struct MockSecretProvider {
    pub secrets: std::sync::Mutex<HashMap<String, String>>,
}

impl MockSecretProvider {
    pub fn new() -> Self {
        Self {
            secrets: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockSecretProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretProvider for MockSecretProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let lock = self.secrets.lock().map_err(|_| "Mutex lock error")?;
        Ok(lock.get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut lock = self.secrets.lock().map_err(|_| "Mutex lock error")?;
        lock.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let mut lock = self.secrets.lock().map_err(|_| "Mutex lock error")?;
        lock.remove(key);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, String> {
        let lock = self.secrets.lock().map_err(|_| "Mutex lock error")?;
        Ok(lock.keys().cloned().collect())
    }
}

// Fallback Crypto / Encryption Module
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedPayload {
    pub salt: String,       // hex encoded
    pub nonce: String,      // hex encoded
    pub ciphertext: String, // hex encoded
}

pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = argon2::Params::new(
        65536, // 64 MB
        3,     // 3 iterations
        4,     // parallelism
        Some(32),
    ).map_err(|e| format!("Argon2 params error: {}", e))?;

    let argon2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    );

    let mut key = [0u8; 32];
    argon2.hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Argon2 hashing failed: {}", e))?;
    Ok(key)
}

pub fn encrypt(passphrase: &str, workspace_id: &str, plaintext: &[u8]) -> Result<EncryptedPayload, String> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill(&mut salt);
    
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);

    let key = derive_key(passphrase, &salt)?;
    let aes_key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let payload = Payload {
        msg: plaintext,
        aad: workspace_id.as_bytes(),
    };

    let ciphertext_bytes = cipher.encrypt(nonce, payload)
        .map_err(|e| format!("AES-256-GCM encryption failed: {}", e))?;

    Ok(EncryptedPayload {
        salt: hex::encode(salt),
        nonce: hex::encode(nonce_bytes),
        ciphertext: hex::encode(ciphertext_bytes),
    })
}

pub fn decrypt(passphrase: &str, workspace_id: &str, payload: &EncryptedPayload) -> Result<Vec<u8>, String> {
    let salt = hex::decode(&payload.salt)
        .map_err(|e| format!("Invalid hex salt: {}", e))?;
    let nonce_bytes = hex::decode(&payload.nonce)
        .map_err(|e| format!("Invalid hex nonce: {}", e))?;
    let ciphertext = hex::decode(&payload.ciphertext)
        .map_err(|e| format!("Invalid hex ciphertext: {}", e))?;

    let key = derive_key(passphrase, &salt)?;
    let aes_key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let payload = Payload {
        msg: &ciphertext,
        aad: workspace_id.as_bytes(),
    };

    let plaintext = cipher.decrypt(nonce, payload)
        .map_err(|e| format!("AES-256-GCM decryption failed: {}", e))?;

    Ok(plaintext)
}

pub fn get_passphrase() -> Result<String, String> {
    if let Ok(key) = std::env::var("FORGE_MASTER_KEY") {
        return Ok(key);
    }
    #[cfg(test)]
    {
        return Err("No master key provided (FORGE_MASTER_KEY not set in test environment)".to_string());
    }
    #[cfg(not(test))]
    {
        print!("Enter Forge master passphrase: ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .map_err(|e| format!("Failed to read passphrase: {}", e))?;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            return Err("Passphrase cannot be empty".to_string());
        }
        Ok(trimmed)
    }
}

pub struct FallbackSecretProvider {
    pub workspace_id: String,
    pub secrets_file_path: PathBuf,
}

impl FallbackSecretProvider {
    pub fn new(workspace_id: &str, path: PathBuf) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            secrets_file_path: path,
        }
    }

    fn read_secrets(&self) -> Result<HashMap<String, String>, String> {
        if !self.secrets_file_path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(&self.secrets_file_path)
            .map_err(|e| format!("Failed to read secrets file: {}", e))?;
        let payload: EncryptedPayload = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid secrets file JSON structure: {}", e))?;

        let passphrase = get_passphrase()?;
        let plaintext = decrypt(&passphrase, &self.workspace_id, &payload)?;
        let secrets: HashMap<String, String> = serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Failed to deserialize decrypted secrets: {}", e))?;
        Ok(secrets)
    }

    fn write_secrets(&self, secrets: &HashMap<String, String>) -> Result<(), String> {
        let plaintext = serde_json::to_vec(secrets)
            .map_err(|e| format!("Failed to serialize secrets: {}", e))?;
        let passphrase = get_passphrase()?;
        let payload = encrypt(&passphrase, &self.workspace_id, &plaintext)?;
        let content = serde_json::to_string_pretty(&payload)
            .map_err(|e| format!("Failed to serialize encrypted payload: {}", e))?;
        if let Some(parent) = self.secrets_file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&self.secrets_file_path, content)
            .map_err(|e| format!("Failed to write secrets file: {}", e))?;
        Ok(())
    }
}

impl SecretProvider for FallbackSecretProvider {
    fn name(&self) -> &str {
        "file"
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let secrets = self.read_secrets()?;
        Ok(secrets.get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut secrets = self.read_secrets()?;
        secrets.insert(key.to_string(), value.to_string());
        self.write_secrets(&secrets)?;
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let mut secrets = self.read_secrets()?;
        secrets.remove(key);
        self.write_secrets(&secrets)?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, String> {
        let secrets = self.read_secrets()?;
        Ok(secrets.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2_and_aes_gcm_roundtrip() {
        let passphrase = "my-secure-master-password";
        let workspace_id = "test-workspace-12345";
        let secret_data = b"{\"API_KEY\": \"super-secret-value-abcd\"}";

        let encrypted = encrypt(passphrase, workspace_id, secret_data).unwrap();
        assert_ne!(encrypted.salt, "");
        assert_ne!(encrypted.nonce, "");
        assert_ne!(encrypted.ciphertext, "");

        let decrypted = decrypt(passphrase, workspace_id, &encrypted).unwrap();
        assert_eq!(decrypted, secret_data);
    }

    #[test]
    fn test_incorrect_aad_fails_decryption() {
        let passphrase = "my-secure-master-password";
        let workspace_id = "test-workspace-12345";
        let secret_data = b"{\"API_KEY\": \"super-secret-value-abcd\"}";

        let encrypted = encrypt(passphrase, workspace_id, secret_data).unwrap();
        
        let wrong_workspace_id = "wrong-workspace-id-54321";
        let decrypt_result = decrypt(passphrase, wrong_workspace_id, &encrypted);
        assert!(decrypt_result.is_err());
    }

    #[test]
    fn test_incorrect_passphrase_fails_decryption() {
        let passphrase = "my-secure-master-password";
        let workspace_id = "test-workspace-12345";
        let secret_data = b"{\"API_KEY\": \"super-secret-value-abcd\"}";

        let encrypted = encrypt(passphrase, workspace_id, secret_data).unwrap();
        
        let wrong_passphrase = "wrong-password";
        let decrypt_result = decrypt(wrong_passphrase, workspace_id, &encrypted);
        assert!(decrypt_result.is_err());
    }

    #[test]
    fn test_ci_bypass_via_env_var() {
        std::env::set_var("FORGE_MASTER_KEY", "ci-secret-bypass");
        let phrase = get_passphrase().unwrap();
        assert_eq!(phrase, "ci-secret-bypass");
        std::env::remove_var("FORGE_MASTER_KEY");
    }
}
