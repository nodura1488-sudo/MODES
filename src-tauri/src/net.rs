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
pub const CATALOG_URL: &str = "https://raw.githubusercontent.com/tommy-whinston/Lavunio-Catalog/main/modes/modes.json";
pub const RAW: &str = "https://raw.githubusercontent.com";

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Lavunio/1.0 (Rust)")
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn download_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(600))
            .user_agent("Lavunio/1.0 (Rust)")
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
    let mut headers = vec![(USER_AGENT.as_str(), "Lavunio".to_string())];
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

/// Читает modes.json из настроенного источника. Сначала catalogUrl, затем GitHub как резерв.
pub fn fetch_remote_modes(
    repo: &str,
    catalog_url: &str,
    token: Option<&str>,
) -> Result<Vec<Mode>, String> {
    let catalog_url = catalog_url.trim();
    let mut catalog_error: Option<String> = None;

    if !catalog_url.is_empty() {
        match fetch_catalog(catalog_url, token) {
            Ok(modes) => return Ok(modes),
            Err(err) => catalog_error = Some(err),
        }
    }

    if repo.trim().is_empty() {
        return match catalog_error {
            Some(err) => Err(err),
            None => Ok(Vec::new()),
        };
    }

    match fetch_repo_modes(repo) {
        Ok(modes) if !modes.is_empty() => Ok(modes),
        Ok(_) => match catalog_error {
            Some(err) => Err(err),
            None => Ok(Vec::new()),
        },
        Err(err) => match catalog_error {
            Some(catalog_err) => Err(format!("{catalog_err}; резервный GitHub: {err}")),
            None => Err(err),
        },
    }
}

fn fetch_catalog(url: &str, token: Option<&str>) -> Result<Vec<Mode>, String> {
    let mut request = client().get(url);
    let github = Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
        .map(|host| {
            host.eq_ignore_ascii_case("raw.githubusercontent.com")
                || host.eq_ignore_ascii_case("api.github.com")
        })
        .unwrap_or(false);
    for (key, value) in auth_headers(token.filter(|_| github)) {
        request = request.header(key, value);
    }
    let res = request
        .send()
        .map_err(|err| format!("Не удалось загрузить каталог: {}", format_error(err)))?;
    let status = res.status();
    if !status.is_success() {
        return Err(format!("Каталог: HTTP {status}"));
    }
    let data: Value = res
        .json()
        .map_err(|e| format!("Не удалось прочитать каталог: {e}"))?;
    parse_modes(data)
}

fn fetch_repo_modes(repo: &str) -> Result<Vec<Mode>, String> {
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
                        .map_err(|e| format!("Не удалось прочитать modes.json: {e}"))?;
                    return parse_modes(data);
                }
                Err(err) => {
                    last_error = Some(format!("Ошибка запроса к GitHub: {}", format_error(err)));
                }
            }
        }
    }
    if not_found {
        return Ok(Vec::new());
    }
    Err(last_error.unwrap_or_else(|| "Не удалось загрузить modes.json".to_string()))
}

fn parse_modes(data: Value) -> Result<Vec<Mode>, String> {
    let Some(items) = data.get("modes").and_then(Value::as_array) else {
        return Err("В каталоге нет массива modes".to_string());
    };
    let mut modes = Vec::new();
    for item in items {
        match serde_json::from_value::<Mode>(item.clone()) {
            Ok(mode) if !mode.id.trim().is_empty() => modes.push(mode),
            _ => {}
        }
    }
    Ok(modes)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FileMeta {
    pub sha256: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ArchivePart {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ArchiveMeta {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub files: u64,
    pub bytes: u64,
    pub parts: Vec<ArchivePart>,
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
        on_event(GameEvent::Status("Режим готов".into()));
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
        on_event(GameEvent::Status("Установка режима".into()));
        for rel in &to_delete {
            if let Some(path) = safe_join(game_dir, rel) {
                let _ = fs::remove_file(path);
            }
        }
        crate::util::prune_empty_dirs(game_dir);
    }

    on_event(GameEvent::Status("Установка режима".into()));
    let parts = if archive.parts.is_empty() {
        vec![ArchivePart {
            path: archive.path.clone(),
            size: archive.size,
            sha256: archive.sha256.clone(),
        }]
    } else {
        archive.parts.clone()
    };
    let total_bytes = parts
        .iter()
        .map(|part| part.size)
        .sum::<u64>()
        .max(archive.size)
        .max(1);
    let temp = std::env::temp_dir().join(format!(
        "lavunio-{}-{}.zip",
        mode.id,
        std::process::id()
    ));
    let mut combined = fs::File::create(&temp)
        .map_err(|e| format!("Не удалось создать файл: {e}"))?;
    let mut offset = 0u64;
    for (index, part) in parts.iter().enumerate() {
        let part_path = std::env::temp_dir().join(format!(
            "lavunio-{}-{}.part{}",
            mode.id,
            std::process::id(),
            index + 1
        ));
        let url = raw_url(
            &settings.github_repo,
            branch,
            &["modes", &mode.id, &part.path],
        )?;
        let mut last_emit = 0u64;
        let download = download_file(
            &url,
            &part_path,
            Some(&settings.github_token),
            cancel,
            |done, _| {
                if done == part.size || done.saturating_sub(last_emit) >= 256 * 1024 {
                    last_emit = done;
                    on_event(GameEvent::Progress {
                        task: offset.saturating_add(done),
                        total: total_bytes,
                    });
                }
            },
        );
        if let Err(err) = download {
            let _ = fs::remove_file(&part_path);
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        let mut downloaded = fs::File::open(&part_path)
            .map_err(|e| format!("Не удалось открыть часть архива: {e}"))?;
        std::io::copy(&mut downloaded, &mut combined)
            .map_err(|e| format!("Не удалось собрать архив: {e}"))?;
        offset = offset.saturating_add(
            fs::metadata(&part_path)
                .map(|meta| meta.len())
                .unwrap_or(part.size),
        );
        let _ = fs::remove_file(&part_path);
        on_event(GameEvent::Progress {
            task: offset,
            total: total_bytes,
        });
    }
    drop(combined);

    if let Some(expected) = archive.sha256.as_deref().filter(|hash| !hash.is_empty()) {
        let actual = sha256_file(&temp)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&temp);
            return Err("Файлы режима повреждены".to_string());
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
    on_event(GameEvent::Status("Проверка режима".into()));

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

    let Some(archive) = manifest.archive.clone() else {
        return Err("Этот режим использует старый формат. Перепубликуйте его в Lavunio Publisher.".to_string());
    };
    sync_build_archive(
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
    )
}


