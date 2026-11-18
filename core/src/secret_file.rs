use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub fn read_secret_file(path: &Path) -> io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

pub fn write_secret_file(path: &Path, secret: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(secret.trim().as_bytes())?;
    file.flush()?;
    Ok(())
}

pub fn delete_secret_file(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn roundtrip_is_private_on_unix() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("prompt-rewriter-secret-{nanos}"));
        let path = dir.join("openrouter.key");
        write_secret_file(&path, "sk-test-value\n").unwrap();
        assert_eq!(read_secret_file(&path).unwrap().as_deref(), Some("sk-test-value"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        delete_secret_file(&path).unwrap();
        assert!(read_secret_file(&path).unwrap().is_none());
        let _ = fs::remove_dir_all(dir);
    }
}
