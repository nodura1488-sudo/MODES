use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::Digest as _;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Не удалось создать папку: {e}"))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp).map_err(|e| format!("Не удалось записать файл: {e}"))?;
        f.write_all(text.as_bytes())
            .map_err(|e| format!("Не удалось записать файл: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("Не удалось сохранить файл: {e}"))?;
    Ok(())
}

pub fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    if rel.trim().is_empty() {
        return None;
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return None;
    }
    if rel_path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(base.join(rel_path))
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Ошибка чтения {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

pub fn prune_empty_dirs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(".manifest.json") {
            continue;
        }
        if path.is_dir() {
            prune_empty_dirs(&path);
            let empty = fs::read_dir(&path)
                .map(|mut it| it.next().is_none())
                .unwrap_or(false);
            if empty {
                let _ = fs::remove_dir(&path);
            }
        }
    }
}

pub fn open_url(url: &str) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Можно открывать только http(s)-ссылки".to_string());
    }
    open::that_detached(url).map_err(|e| format!("Не удалось открыть ссылку: {e}"))
}

pub fn open_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| format!("Не удалось создать папку: {e}"))?;
    }
    open::that_detached(path).map_err(|e| format!("Не удалось открыть папку: {e}"))
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn format_error(err: impl std::fmt::Display) -> String {
    let s = err.to_string();
    let s = s.trim();
    if s.is_empty() {
        "Неизвестная ошибка".to_string()
    } else {
        s.to_string()
    }
}
