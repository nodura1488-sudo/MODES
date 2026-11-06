use crate::events::GameEvent;
use crate::models::{Mode, Settings};
use crate::util::{format_error, safe_join, sha256_file, write_json};
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use url::Url;

pub const API: &str = "https://api.github.com";
pub const RAW: &str = "https://raw.githubusercontent.com";

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("TM-Launcher/0.3 (Rust)")
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn download_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(600))
            .user_agent("TM-Launcher/0.3 (Rust)")
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

pub fn raw_url(repo: &str, branch: &str, segments: &[&str]) -> Result<String, String> {
    let mut url = Url::parse(&format!("{RAW}/{repo}/{branch}/"))
        .map_err(|e| format!("Некорректный адрес GitHub: {e}"))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "Некорректный адрес GitHub".to_string())?;
        for segment in segments {
            path.push(segment);
        }
    }
    Ok(url.to_string())
}

pub fn api_url(repo: &str, tail: &[&str]) -> Result<Url, String> {
    let mut url = Url::parse(&format!("{API}/repos/{repo}/"))
        .map_err(|e| format!("Некорректный репозиторий: {e}"))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "Некорректный репозиторий".to_string())?;
        for segment in tail {
            path.push(segment);
        }
    }
    Ok(url)
}

fn auth_headers(token: Option<&str>) -> Vec<(&'static str, String)> {
    let mut headers = vec![(USER_AGENT.as_str(), "TM-Launcher".to_string())];
    if let Some(token) = token.filter(|t| !t.trim().is_empty()) {
        headers.push((AUTHORIZATION.as_str(), format!("Bearer {}", token.trim())));
    }
    headers
}

fn get_json(url: &str, token: Option<&str>) -> Result<Value, String> {
    let mut req = client()
        .get(url)
        .header(ACCEPT, "application/vnd.github+json");
    for (k, v) in auth_headers(token) {
        req = req.header(k, v);
    }
    let res = req
        .send()
        .map_err(|e| format!("Нет связи с GitHub: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| Value::Null);
    if !status.is_success() {
        let message = data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("ошибка GitHub API");
        return Err(format!("GitHub API ({status}): {message}"));
    }
    Ok(data)
}

/// Читает modes.json из репозитория. Сеть недоступна -> Err, чтобы приложение взяло кэш.
pub fn fetch_remote_modes(repo: &str) -> Result<Vec<Mode>, String> {
    if repo.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut not_found = false;
    let mut last_error: Option<String> = None;
    for branch in ["main", "master"] {
        for segments in [&["modes", "modes.json"][..], &["modes.json"][..]] {
            let url = raw_url(repo, branch, segments)?;
            match client().get(&url).send() {
                Ok(res) => {
                    let status = res.status();
                    if status == reqwest::StatusCode::NOT_FOUND {
                        not_found = true;
                        continue;
                    }
                    if !status.is_success() {
                        last_error = Some(format!("GitHub ответил {status}"));
                        continue;
                    }
                    let data: Value = res
                        .json()
                        .map_err(|e| format!("Некорректный modes.json: {e}"))?;
                    let Some(items) = data.get("modes").and_then(Value::as_array) else {
                        return Err("В modes.json нет массива modes".to_string());
                    };
                    let mut modes = Vec::new();
                    for item in items {
                        match serde_json::from_value::<Mode>(item.clone()) {
                            Ok(mode) if !mode.id.trim().is_empty() => modes.push(mode),
                            _ => {}
                        }
                    }
                    return Ok(modes);
                }
                Err(err) => {
                    last_error = Some(format!("Нет связи с GitHub: {}", format_error(err)));
                }
            }
        }
    }
    if not_found {
        return Ok(Vec::new());
    }
    Err(last_error.unwrap_or_else(|| "Не удалось прочитать modes.json".to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FileMeta {
    pub sha256: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ArchiveMeta {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Manifest {
    pub mode: String,
    pub version: String,
    pub generated_at: Option<String>,
    pub archive: Option<ArchiveMeta>,
    pub files: HashMap<String, FileMeta>,
    #[serde(skip)]
    pub branch: String,
}

fn parse_manifest(value: Value) -> Result<Manifest, String> {
    let mut manifest: Manifest =
        serde_json::from_value(value).map_err(|e| format!("Некорректный manifest.json: {e}"))?;
    // На всякий случай нормализуем относительные пути.
    manifest.files = manifest
        .files
        .into_iter()
        .filter_map(|(path, meta)| {
            let clean = path.replace('\\', "/");
            let clean = clean.trim_start_matches("./").to_string();
            if clean.is_empty() || clean == "manifest.json" {
                None
            } else {
                Some((clean, meta))
            }
        })
        .collect();
    if let Some(archive) = manifest.archive.as_mut() {
        archive.path = archive
            .path
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string();
        if archive.path.is_empty() {
            archive.path = "build.zip".to_string();
        }
    }
    Ok(manifest)
}

pub fn fetch_manifest(
    repo: &str,
    token: Option<&str>,
    mode_id: &str,
    mode_title: &str,
) -> Result<Manifest, String> {
    let mut last_error: Option<String> = None;

    for branch in ["main", "master"] {
        let url = raw_url(repo, branch, &["modes", mode_id, "manifest.json"])?;
        let mut req = client().get(&url);
        for (k, v) in auth_headers(token) {
            req = req.header(k, v);
        }
        match req.send() {
            Ok(res) => {
                if res.status().is_success() {
                    let data: Value = res
                        .json()
                        .map_err(|e| format!("Некорректный manifest.json: {e}"))?;
                    let mut manifest = parse_manifest(data)?;
                    manifest.branch = branch.to_string();
                    return Ok(manifest);
                }
                last_error = Some(format!("manifest.json ({})", res.status()));
            }
            Err(err) => last_error = Some(format_error(err)),
        }
    }

    // Fallback: если сборку залили руками, собираем список по git tree.
    for branch in ["main", "master"] {
        let mut url = api_url(repo, &["git", "trees", branch])?;
        url.query_pairs_mut().append_pair("recursive", "1");
        match get_json(url.as_str(), token) {
            Ok(data) => {
                let prefix = format!("modes/{mode_id}/");
                let mut files = HashMap::new();
                if let Some(tree) = data.get("tree").and_then(Value::as_array) {
                    for item in tree {
                        if item.get("type").and_then(Value::as_str) != Some("blob") {
                            continue;
                        }
                        let Some(path) = item.get("path").and_then(Value::as_str) else {
                            continue;
                        };
                        let Some(rel) = path.strip_prefix(&prefix) else {
                            continue;
                        };
                        if rel.is_empty() || rel == "manifest.json" {
                            continue;
                        }
                        files.insert(
                            rel.to_string(),
                            FileMeta {
                                sha256: None,
                                size: item.get("size").and_then(Value::as_u64).unwrap_or(0),
                            },
                        );
                    }
                }
                if !files.is_empty() {
                    return Ok(Manifest {
                        mode: mode_id.to_string(),
                        version: String::new(),
                        generated_at: None,
                        archive: None,
                        files,
                        branch: branch.to_string(),
                    });
                }
                last_error = Some(format!("в репозитории нет папки modes/{mode_id}/"));
            }
            Err(err) => last_error = Some(err),
        }
    }

    Err(format!(
        "Сборка «{mode_title}» не найдена в репозитории. Залей файлы в modes/{mode_id}/ или публикуй кнопкой «+».{}",
        last_error
            .map(|e| format!(" ({e})"))
            .unwrap_or_default()
    ))
}

pub fn download_file(
    url: &str,
    dest: &Path,
    token: Option<&str>,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Операция отменена".to_string());
    }
    let mut req = download_client().get(url);
    for (k, v) in auth_headers(token) {
        req = req.header(k, v);
    }
    let mut res: Response = req
        .send()
        .map_err(|e| format!("Ошибка скачивания: {}", format_error(e)))?;
    let status = res.status();
    if !status.is_success() {
        return Err(format!(
            "Ошибка скачивания {} ({status})",
            dest.file_name().and_then(|n| n.to_str()).unwrap_or("файла")
        ));
    }
    let total = res.content_length().unwrap_or(0);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Не удалось создать папку: {e}"))?;
    }
    let part = dest.with_extension(format!(
        "{}tm-part",
        dest.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}."))
            .unwrap_or_default()
    ));
    let result = (|| -> Result<(), String> {
        let mut file = File::create(&part)
            .map_err(|e| format!("Не удалось создать {}: {e}", part.display()))?;
        let mut buf = [0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Операция отменена".to_string());
            }
            let n = res
                .read(&mut buf)
                .map_err(|e| format!("Ошибка чтения ответа: {e}"))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])
                .map_err(|e| format!("Ошибка записи файла: {e}"))?;
            done += n as u64;
            on_progress(done, total.max(done));
        }
        file.flush()
            .map_err(|e| format!("Ошибка записи файла: {e}"))?;
        Ok(())
    })();
    if let Err(err) = result {
        let _ = fs::remove_file(&part);
        return Err(err);
    }
    fs::rename(&part, dest).map_err(|e| format!("Не удалось переименовать файл: {e}"))?;
    Ok(())
}

/// Инкрементальный синк сборки по манифесту: качаем изменённое, удаляем лишнее.
fn extract_archive(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Не удалось открыть архив: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Не удалось прочитать архив: {e}"))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("Ошибка файла архива: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let Some(out) = safe_join(dest, &relative) else {
            continue;
        };
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать папку: {e}"))?;
        }
        let mut target = fs::File::create(&out)
            .map_err(|e| format!("Не удалось создать файл: {e}"))?;
        std::io::copy(&mut entry, &mut target)
            .map_err(|e| format!("Не удалось распаковать файл: {e}"))?;
    }
    Ok(())
}

fn sync_build_archive(
    mode: &Mode,
    settings: &Settings,
    game_dir: &Path,
    manifest: &Manifest,
    archive: &ArchiveMeta,
    branch: &str,
    local_path: &Path,
    local: &Manifest,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(GameEvent),
) -> Result<(), String> {
    let same = local.version == manifest.version
        && local.archive.as_ref().and_then(|item| item.sha256.as_deref())
            == archive.sha256.as_deref()
        && local.files.len() == manifest.files.len();
    if same {
        on_event(GameEvent::Status("Сборка готова".into()));
        return Ok(());
    }

    let mut to_delete: Vec<String> = local
        .files
        .keys()
        .filter(|rel| !manifest.files.contains_key(*rel))
        .cloned()
        .collect();
    to_delete.sort();
    if !to_delete.is_empty() {
        on_event(GameEvent::Status("Установка сборки".into()));
        for rel in &to_delete {
            if let Some(path) = safe_join(game_dir, rel) {
                let _ = fs::remove_file(path);
            }
        }
        crate::util::prune_empty_dirs(game_dir);
    }

    on_event(GameEvent::Status("Установка сборки".into()));
    let temp = std::env::temp_dir().join(format!(
        "tmstore-{}-{}.zip",
        mode.id,
        std::process::id()
    ));
    let url = raw_url(&settings.github_repo, branch, &["modes", &mode.id, &archive.path])?;
    let mut last_emit = 0u64;
    let download = download_file(
        &url,
        &temp,
        Some(&settings.github_token),
        cancel,
        |done, total| {
            if done == total || done.saturating_sub(last_emit) >= 256 * 1024 {
                last_emit = done;
                on_event(GameEvent::Progress {
                    task: done,
                    total: total.max(done),
                });
            }
        },
    );
    if let Err(err) = download {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }

    if let Some(expected) = archive.sha256.as_deref().filter(|hash| !hash.is_empty()) {
        let actual = sha256_file(&temp)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&temp);
            return Err("Файлы сборки повреждены".to_string());
        }
    }

    on_event(GameEvent::Status("Распаковка файлов".into()));
    let extracted = extract_archive(&temp, game_dir);
    let _ = fs::remove_file(&temp);
    extracted?;
    write_json(local_path, manifest)?;
    Ok(())
}

pub fn sync_build(
    mode: &Mode,
    settings: &Settings,
    game_dir: &Path,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(GameEvent),
) -> Result<(), String> {
    on_event(GameEvent::Status("Проверка сборки".into()));

    let mut manifest = fetch_manifest(
        &settings.github_repo,
        Some(&settings.github_token),
        &mode.id,
        &mode.title,
    )?;
    if manifest.version.is_empty() {
        manifest.version = mode.version.clone();
    }
    let branch = if manifest.branch.is_empty() {
        "main".to_string()
    } else {
        manifest.branch.clone()
    };
    let local_path = game_dir.join(".manifest.json");
    let local: Manifest = crate::util::read_json(&local_path).unwrap_or_default();

    if let Some(archive) = manifest.archive.clone() {
        return sync_build_archive(
            mode,
            settings,
            game_dir,
            &manifest,
            &archive,
            &branch,
            &local_path,
            &local,
            cancel,
            on_event,
        );
    }

    let mut to_download: Vec<String> = Vec::new();
    for (rel, meta) in &manifest.files {
        let same = local.files.get(rel).map_or(false, |lf| {
            meta.sha256.is_none() || lf.sha256.as_deref() == meta.sha256.as_deref()
        });
        if !same {
            to_download.push(rel.clone());
        }
    }
    to_download.sort();
    let mut to_delete: Vec<String> = local
        .files
        .keys()
        .filter(|rel| !manifest.files.contains_key(*rel))
        .cloned()
        .collect();
    to_delete.sort();

    if to_download.is_empty() && to_delete.is_empty() {
        on_event(GameEvent::Status("Сборка готова".into()));
        write_json(&local_path, &manifest)?;
        return Ok(());
    }

    let total_bytes: u64 = to_download
        .iter()
        .map(|rel| manifest.files.get(rel).map(|m| m.size).unwrap_or(0))
        .sum();
    let mut done_bytes = 0u64;

    on_event(GameEvent::Status("Установка сборки".into()));
    for rel in &to_download {
        if cancel.load(Ordering::Relaxed) {
            return Err("Операция отменена".to_string());
        }
        let Some(dest) = safe_join(game_dir, rel) else {
            return Err(format!("Опасный путь в манифесте: {rel}"));
        };
        let url = raw_url(&settings.github_repo, &branch, &["modes", &mode.id, rel])?;
        let meta = manifest.files.get(rel).cloned().unwrap_or_default();
        let mut last_task = 0u64;
        download_file(
            &url,
            &dest,
            Some(&settings.github_token),
            cancel,
            |done, total| {
                let base = done_bytes;
                let _ = total;
                if done.saturating_sub(last_task) >= 256 * 1024 || done == meta.size {
                    last_task = done;
                    on_event(GameEvent::Progress {
                        task: base + done,
                        total: total_bytes.max(base + done),
                    });
                }
            },
        )?;
        if let Some(expected) = meta.sha256.as_deref().filter(|s| !s.is_empty()) {
            let hash = sha256_file(&dest)?;
            if !hash.eq_ignore_ascii_case(expected) {
                let _ = fs::remove_file(&dest);
                return Err(format!("Файл повреждён при скачивании: {rel}"));
            }
        }
        done_bytes += meta.size;
        on_event(GameEvent::Progress {
            task: done_bytes,
            total: total_bytes.max(done_bytes),
        });
    }

    if !to_delete.is_empty() {
        on_event(GameEvent::Status("Установка сборки".into()));
        for rel in &to_delete {
            if let Some(path) = safe_join(game_dir, rel) {
                let _ = fs::remove_file(path);
            }
        }
        crate::util::prune_empty_dirs(game_dir);
    }

    write_json(&local_path, &manifest)?;
    Ok(())
}


