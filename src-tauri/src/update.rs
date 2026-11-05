use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use std::time::Duration;

const REPO: &str = "tommy-whinston/TMLauncher";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current: String,
    pub latest: String,
    pub url: String,
    pub notes: String,
    pub installer_url: Option<String>,
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("TMLauncher/0.4 (Rust)")
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .take(3)
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let mut left = version_parts(latest);
    let mut right = version_parts(current);
    while left.len() < 3 {
        left.push(0);
    }
    while right.len() < 3 {
        right.push(0);
    }
    left > right
}

#[tauri::command]
pub async fn check_update() -> Result<UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
        let response = client()
            .get(&url)
            .send()
            .map_err(|e| format!("Не удалось проверить обновление: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("GitHub ответил {}", response.status()));
        }
        let data: Value = response
            .json()
            .map_err(|e| format!("Некорректный ответ GitHub: {e}"))?;

        let latest = data
            .get("tag_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .trim_start_matches(['v', 'V'])
            .to_string();
        let current = env!("CARGO_PKG_VERSION").to_string();
        let html_url = data
            .get("html_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let notes = data
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let installer_url = data
            .get("assets")
            .and_then(Value::as_array)
            .and_then(|assets| {
                assets.iter().find_map(|asset| {
                    let name = asset.get("name").and_then(Value::as_str)?.to_lowercase();
                    if !name.contains("setup") || !name.ends_with(".exe") {
                        return None;
                    }
                    asset
                        .get("browser_download_url")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
            });

        Ok(UpdateInfo {
            available: !latest.is_empty() && is_newer(&latest, &current),
            current,
            latest,
            url: html_url,
            notes,
            installer_url,
        })
    })
    .await
    .map_err(|e| format!("Не удалось проверить обновление: {e}"))?
}

#[tauri::command]
pub async fn download_update(url: String) -> Result<String, String> {
    if !url.starts_with("https://github.com/") {
        return Err("Некорректная ссылка на обновление".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let path: PathBuf = std::env::temp_dir().join("TMLauncher-Setup-latest.exe");
        let mut response = client()
            .get(&url)
            .send()
            .map_err(|e| format!("Не удалось скачать установщик: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("GitHub ответил {}", response.status()));
        }
        let mut file =
            File::create(&path).map_err(|e| format!("Не удалось создать файл: {e}"))?;
        copy(&mut response, &mut file).map_err(|e| format!("Не удалось сохранить: {e}"))?;
        drop(file);

        open::that(&path).map_err(|e| format!("Не удалось запустить установщик: {e}"))?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("Не удалось скачать обновление: {e}"))?
}
