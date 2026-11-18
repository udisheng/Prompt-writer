use rewriter_core::{delete_secret_file, read_secret_file, write_secret_file};
use std::path::Path;

const SERVICE: &str = "com.promptrewriter.app";
const ACCOUNT: &str = "openrouter";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStore {
    OsKeychain,
    PrivateFile,
}

impl SecretStore {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OsKeychain => "os-keychain",
            Self::PrivateFile => "private-file",
        }
    }
}

pub struct SecretStatus {
    pub configured: bool,
    pub store: SecretStore,
}

pub fn secret_status(path: &Path) -> SecretStatus {
    let keychain_ok = keychain_works();
    let in_keychain = keychain_ok && keychain_get().ok().flatten().is_some();
    let in_file = read_secret_file(path).ok().flatten().is_some();
    if in_keychain {
        SecretStatus {
            configured: true,
            store: SecretStore::OsKeychain,
        }
    } else if in_file {
        SecretStatus {
            configured: true,
            store: SecretStore::PrivateFile,
        }
    } else if keychain_ok {
        SecretStatus {
            configured: false,
            store: SecretStore::OsKeychain,
        }
    } else {
        SecretStatus {
            configured: false,
            store: SecretStore::PrivateFile,
        }
    }
}

pub fn load_key(path: &Path) -> Result<Option<String>, String> {
    if keychain_works() {
        match keychain_get() {
            Ok(Some(key)) => return Ok(Some(key)),
            Ok(None) => {}
            Err(err) => {
                eprintln!("keychain read failed, trying the private file: {err}");
            }
        }
    }
    read_secret_file(path).map_err(|err| format!("Could not read the saved API key: {err}"))
}

pub fn save_key(path: &Path, key: &str) -> Result<SecretStore, String> {
    let key = validate_key(key)?;
    if keychain_works() {
        match keychain_set(&key) {
            Ok(()) => {
                let _ = delete_secret_file(path);
                return Ok(SecretStore::OsKeychain);
            }
            Err(err) => {
                eprintln!("keychain write failed, using the private file: {err}");
            }
        }
    }
    write_secret_file(path, &key).map_err(|err| format!("Could not store the API key: {err}"))?;
    Ok(SecretStore::PrivateFile)
}

pub fn clear_key(path: &Path) -> Result<(), String> {
    if let Err(err) = keychain_delete() {
        eprintln!("keychain delete failed: {err}");
    }
    delete_secret_file(path).map_err(|err| format!("Could not remove the saved API key: {err}"))
}

fn validate_key(key: &str) -> Result<String, String> {
    let key = key.trim();
    if key.chars().any(char::is_whitespace) || key.len() < 8 || key.len() > 512 {
        return Err("Paste the API key as a single token, with no spaces.".into());
    }
    Ok(key.to_string())
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|err| err.to_string())
}

fn keychain_works() -> bool {
    match entry() {
        Ok(entry) => match entry.get_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn keychain_get() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn keychain_set(key: &str) -> Result<(), String> {
    entry()?.set_password(key).map_err(|err| err.to_string())
}

fn keychain_delete() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}
