//! API keys for cloud services. A key never goes in a project or a settings file: it lives in
//! the OS credential store (Windows Credential Manager, macOS Keychain), or in an owner-only
//! file on other platforms. An environment variable overrides the stored key.
use std::path::Path;

/// One service's key: the environment variable that overrides it and its storage name.
#[derive(Clone, Copy, Debug)]
pub struct KeySpec {
    pub env: &'static str,
    /// Keychain account name, and the file stem on platforms without a keychain.
    pub name: &'static str,
}

#[cfg(any(windows, target_os = "macos"))]
const KEYRING_SERVICE: &str = "AeroEdits";

/// The key and where it came from: `environment`, `keychain` or `file`.
pub fn get(dir: &Path, spec: KeySpec) -> Option<(String, &'static str)> {
    if let Ok(key) = std::env::var(spec.env) {
        if !key.trim().is_empty() {
            return Some((key.trim().to_string(), "environment"));
        }
    }
    stored(dir, spec)
}

#[cfg(any(windows, target_os = "macos"))]
fn stored(_dir: &Path, spec: KeySpec) -> Option<(String, &'static str)> {
    keyring::Entry::new(KEYRING_SERVICE, spec.name)
        .ok()?
        .get_password()
        .ok()
        .filter(|k| !k.is_empty())
        .map(|k| (k, "keychain"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn stored(dir: &Path, spec: KeySpec) -> Option<(String, &'static str)> {
    std::fs::read_to_string(dir.join(format!("{}.key", file_stem(spec))))
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .map(|k| (k, "file"))
}

/// The file name keeps the old `elevenlabs.key` for the ElevenLabs key.
#[cfg(not(any(windows, target_os = "macos")))]
fn file_stem(spec: KeySpec) -> &'static str {
    spec.name.strip_suffix("-api-key").unwrap_or(spec.name)
}

/// Stores `key`, or removes the stored key when `key` is empty.
pub fn set(dir: &Path, spec: KeySpec, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.len() > 512 || key.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("That does not look like an API key".into());
    }
    store(dir, spec, key)
}

#[cfg(any(windows, target_os = "macos"))]
fn store(_dir: &Path, spec: KeySpec, key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, spec.name).map_err(|e| e.to_string())?;
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        entry.set_password(key).map_err(|e| e.to_string())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn store(dir: &Path, spec: KeySpec, key: &str) -> Result<(), String> {
    use std::fs;
    let path = dir.join(format!("{}.key", file_stem(spec)));
    if key.is_empty() {
        return match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut file, key.as_bytes()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: KeySpec = KeySpec {
        env: "AEROEDITS_TEST_UNSET_KEY_ENV",
        name: "test-api-key",
    };

    #[test]
    fn key_must_be_a_single_token() {
        let dir = tempfile::tempdir().unwrap();
        assert!(set(dir.path(), SPEC, "sk a").is_err());
        assert!(set(dir.path(), SPEC, "sk\u{7}").is_err());
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn file_key_store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), SPEC, "sk_test").unwrap();
        assert_eq!(get(dir.path(), SPEC).unwrap(), ("sk_test".into(), "file"));
        assert!(dir.path().join("test.key").is_file());
        set(dir.path(), SPEC, "").unwrap();
        assert!(get(dir.path(), SPEC).is_none());
    }
}
