use crate::events::PublishEvent;
use crate::models::Mode;
use crate::net::{
    api_delete_json, api_put_json, choose_branch, contents_json, contents_url, raw_url, repo_tree,
    Manifest, FileMeta,
};
use crate::util::{
    git_blob_sha1, human_size, is_junk, now_ms, sha256_file, temp_dir, write_json,
};
use base64::Engine as _;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct PublishForm {
    pub repo: String,
    pub token: String,
    pub title: String,
    pub version: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub tags: Vec<String>,
    pub description: String,
    pub changelog: String,
    pub accent: String,
    pub image_path: Option<PathBuf>,
    pub zip_path: Option<PathBuf>,
}

pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    for ch in title.to_lowercase().chars() {
        let mapped: String = match ch {
            'а' => "a".into(),
            'б' => "b".into(),
            'в' => "v".into(),
            'г' => "g".into(),
            'д' => "d".into(),
            'е' => "e".into(),
            'ё' => "e".into(),
            'ж' => "zh".into(),
            'з' => "z".into(),
            'и' => "i".into(),
            'й' => "y".into(),
            'к' => "k".into(),
            'л' => "l".into(),
            'м' => "m".into(),
            'н' => "n".into(),
            'о' => "o".into(),
            'п' => "p".into(),
            'р' => "r".into(),
            'с' => "s".into(),
            'т' => "t".into(),
            'у' => "u".into(),
            'ф' => "f".into(),
            'х' => "h".into(),
            'ц' => "c".into(),
            'ч' => "ch".into(),
            'ш' => "sh".into(),
            'щ' => "sch".into(),
            'ъ' => String::new(),
            'ы' => "y".into(),
            'ь' => String::new(),
            'э' => "e".into(),
            'ю' => "yu".into(),
            'я' => "ya".into(),
            other => other.to_string(),
        };
        for c in mapped.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c);
            } else if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "mode".to_string()
    } else {
        out
    }
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    parts.next().is_none()
        && !owner.is_empty()
        && !name.is_empty()
        && owner.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn emit(tx: &Sender<crate::events::AppEvent>, event: PublishEvent) {
    let _ = tx.send(crate::events::AppEvent::Publish(event));
}

fn step(tx: &Sender<crate::events::AppEvent>, ctx: &egui::Context, text: impl Into<String>) {
    emit(tx, PublishEvent::Step(text.into()));
    ctx.request_repaint();
}

fn progress(tx: &Sender<crate::events::AppEvent>, ctx: &egui::Context, task: u64, total: u64) {
    emit(tx, PublishEvent::Progress { task, total });
    ctx.request_repaint();
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Не удалось открыть zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Некорректный zip: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Ошибка чтения zip: {e}"))?;
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        if rel
            .components()
            .any(|c| is_junk(&c.as_os_str().to_string_lossy()))
        {
            continue;
        }
        let out = dest.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| format!("Не удалось создать папку: {e}"))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Не удалось создать папку: {e}"))?;
        }
        let mut f = File::create(&out).map_err(|e| format!("Не удалось создать файл: {e}"))?;
        copy(&mut entry, &mut f).map_err(|e| format!("Ошибка распаковки: {e}"))?;
    }
    Ok(())
}

fn walk_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        !is_junk(&e.file_name().to_string_lossy())
    });
    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        out.push((entry.path().to_path_buf(), rel));
    }
    out
}

fn put_file(
    repo: &str,
    branch: &str,
    token: &str,
    repo_path: &str,
    abs_path: &Path,
    existing: &HashMap<String, String>,
) -> Result<(), String> {
    let sha = git_blob_sha1(abs_path)?;
    if existing.get(repo_path).map(String::as_str) == Some(sha.as_str()) {
        return Ok(());
    }
    let bytes = fs::read(abs_path).map_err(|e| format!("Не удалось прочитать файл: {e}"))?;
    let content = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut body = Map::new();
    body.insert("message".into(), json!(format!("build: {repo_path}")));
    body.insert("content".into(), json!(content));
    body.insert("branch".into(), json!(branch));
    if let Some(old_sha) = existing.get(repo_path) {
        body.insert("sha".into(), json!(old_sha));
    }
    let url = contents_url(repo, repo_path)?;
    let mut last_error = String::new();
    for attempt in 1..=3 {
        match api_put_json(&url, token, &Value::Object(body.clone())) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_error = err;
                if attempt < 3 {
                    sleep(Duration::from_millis(800 * attempt));
                }
            }
        }
    }
    Err(last_error)
}

fn delete_file(repo: &str, branch: &str, token: &str, repo_path: &str, sha: &str) {
    let Ok(url) = contents_url(repo, repo_path) else {
        return;
    };
    let body = json!({
        "message": format!("build: remove {repo_path}"),
        "sha": sha,
        "branch": branch
    });
    let _ = api_delete_json(&url, token, &body);
}

pub fn publish(
    form: &PublishForm,
    tx: &Sender<crate::events::AppEvent>,
    ctx: &egui::Context,
) -> Result<Mode, String> {
    let fail = |message: String| message;

    step(tx, ctx, "Проверяю данные…");
    if !valid_repo(&form.repo) {
        return Err(fail("Репозиторий в формате owner/repo, например nodura1488-sudo/MODES".into()));
    }
    if !valid_token(&form.token) {
        return Err(fail("Нужен GitHub токен (ghp_… или github_pat_…)".into()));
    }
    if form.title.trim().is_empty() {
        return Err(fail("Введи название режима".into()));
    }
    if !form
        .version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        || form.version.trim().is_empty()
    {
        return Err(fail("Версия: только буквы, цифры, точки и дефисы".into()));
    }
    if !form
        .mc_version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        || form.mc_version.trim().is_empty()
    {
        return Err(fail("Версия Minecraft указана неверно".into()));
    }
    if !matches!(form.loader.as_str(), "vanilla" | "forge" | "fabric") {
        return Err(fail("Загрузчик: vanilla, forge или fabric".into()));
    }

    let id = slugify(&form.title);
    let build_prefix = format!("modes/{id}");
    let tmp = temp_dir("publish");
    fs::create_dir_all(&tmp).map_err(|e| fail(format!("Не удалось создать временную папку: {e}")))?;

    let mut files: HashMap<String, FileMeta> = HashMap::new();
    let mut total_bytes = 0u64;
    if let Some(zip_path) = form.zip_path.as_deref() {
        step(tx, ctx, "Читаю сборку…");
        if let Err(err) = extract_zip(zip_path, &tmp) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(fail(err));
        }
        let list = walk_files(&tmp);
        step(tx, ctx, format!("Считаю хэши ({} файлов)…", list.len()));
        for (abs, rel) in list {
            if rel == "manifest.json" {
                continue;
            }
            let meta = fs::metadata(&abs)
                .map_err(|e| fail(format!("Не удалось прочитать {}: {e}", rel)))?;
            if meta.len() > 95 * 1024 * 1024 {
                let _ = fs::remove_dir_all(&tmp);
                return Err(fail(format!(
                    "Файл больше 95 МБ: {rel}. GitHub Contents API не примет — убери или залей вручную"
                )));
            }
            let hash = sha256_file(&abs)?;
            total_bytes += meta.len();
            files.insert(
                rel,
                FileMeta {
                    sha256: Some(hash),
                    size: meta.len(),
                },
            );
        }
    }

    let manifest = Manifest {
        mode: id.clone(),
        version: form.version.clone(),
        generated_at: Some(format!("{}", now_ms())),
        files: files.clone(),
        branch: String::new(),
    };

    step(tx, ctx, "Читаю репозиторий…");
    let branch = choose_branch(&form.repo, Some(&form.token))?;
    let existing = match repo_tree(&form.repo, &branch, Some(&form.token)) {
        Ok(tree) => tree,
        Err(err) if err.contains("409") || err.contains("404") => HashMap::new(),
        Err(err) => {
            let _ = fs::remove_dir_all(&tmp);
            return Err(fail(err));
        }
    };

    let old_build_paths: Vec<String> = existing
        .keys()
        .filter(|p| p.starts_with(&format!("{build_prefix}/")))
        .cloned()
        .collect();
    let new_paths: HashSet<String> = files
        .keys()
        .map(|p| p.clone())
        .chain(std::iter::once("manifest.json".to_string()))
        .collect();
    let to_delete: Vec<String> = old_build_paths
        .into_iter()
        .filter(|p| {
            let rel = p
                .strip_prefix(&format!("{build_prefix}/"))
                .unwrap_or_default();
            !new_paths.contains(rel)
        })
        .collect();

    let file_entries: Vec<(String, FileMeta)> = files.clone().into_iter().collect();
    let count = file_entries.len();
    for (index, (rel, _meta)) in file_entries.iter().enumerate() {
        step(
            tx,
            ctx,
            format!("Загрузка {}/{}: {rel}", index + 1, count),
        );
        progress(tx, ctx, index as u64, count as u64);
        let repo_path = format!("{build_prefix}/{rel}");
        if let Err(err) = put_file(
            &form.repo,
            &branch,
            &form.token,
            &repo_path,
            &tmp.join(rel),
            &existing,
        ) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(fail(err));
        }
    }
    progress(tx, ctx, count as u64, count as u64);

    step(tx, ctx, "Загружаю манифест…");
    let manifest_tmp = tmp.join("manifest.json");
    write_json(&manifest_tmp, &manifest)?;
    if let Err(err) = put_file(
        &form.repo,
        &branch,
        &form.token,
        &format!("{build_prefix}/manifest.json"),
        &manifest_tmp,
        &existing,
    ) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(fail(err));
    }

    if !to_delete.is_empty() {
        step(
            tx,
            ctx,
            format!("Удаляю устаревшие файлы ({})…", to_delete.len()),
        );
        for repo_path in &to_delete {
            if let Some(sha) = existing.get(repo_path) {
                delete_file(&form.repo, &branch, &form.token, repo_path, sha);
            }
        }
    }

    let mut image_url: Option<String> = None;
    if let Some(image_path) = form.image_path.as_deref() {
        step(tx, ctx, "Загружаю арт…");
        let ext = image_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "png".to_string());
        let art_path = format!("arts/{id}.{ext}");
        if let Err(err) = put_file(
            &form.repo,
            &branch,
            &form.token,
            &art_path,
            image_path,
            &existing,
        ) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(fail(err));
        }
        image_url = raw_url(
            &form.repo,
            &branch,
            &["arts", &format!("{id}.{ext}")],
        )
        .ok();
    }

    step(tx, ctx, "Обновляю modes.json…");
    let (_modes_sha, remote_modes) = match contents_json(&form.repo, "modes.json", &branch, &form.token) {
        Ok((sha, content)) => {
            let text = base64::engine::general_purpose::STANDARD
                .decode(content)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();
            let modes = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("modes").and_then(Value::as_array).cloned())
                .unwrap_or_default();
            (Some(sha), modes)
        }
        Err(err) if err.contains("404") => (None, Vec::new()),
        Err(err) => {
            let _ = fs::remove_dir_all(&tmp);
            return Err(fail(err));
        }
    };

    let entry = json!({
        "id": id,
        "title": form.title.trim(),
        "mcVersion": form.mc_version.trim(),
        "version": form.version.trim(),
        "loader": form.loader,
        "loaderVersion": if form.loader == "vanilla" { Value::Null } else { json!(form.loader_version.as_deref().filter(|v| !v.is_empty())) },
        "size": if form.zip_path.is_some() { json!(human_size(total_bytes)) } else { json!("—") },
        "players": "—",
        "accent": form.accent,
        "tags": if form.tags.is_empty() { json!(["Режим"]) } else { json!(form.tags) },
        "image": "",
        "imageUrl": image_url,
        "build": {
            "files": files.len(),
            "bytes": total_bytes,
            "version": form.version.trim()
        },
        "description": form.description,
        "changelog": form.changelog
    });

    let mut merged: Vec<Value> = Vec::new();
    let mut found = false;
    for mut old in remote_modes {
        let old_id = old.get("id").and_then(Value::as_str).unwrap_or_default();
        if old_id == id {
            if let (Some(old_obj), Some(new_obj)) = (old.as_object_mut(), entry.as_object()) {
                for (k, v) in new_obj {
                    old_obj.insert(k.clone(), v.clone());
                }
            }
            found = true;
        }
        merged.push(old);
    }
    if !found {
        merged.push(entry.clone());
    }

    let modes_tmp = tmp.join("modes.json");
    write_json(&modes_tmp, &json!({ "modes": merged }))?;
    if let Err(err) = put_file(
        &form.repo,
        &branch,
        &form.token,
        "modes.json",
        &modes_tmp,
        &existing,
    ) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(fail(err));
    }

    let _ = fs::remove_dir_all(&tmp);

    let mode: Mode = serde_json::from_value(entry).map_err(|e| fail(format!("Не удалось собрать режим: {e}")))?;
    emit(tx, PublishEvent::Done(mode.clone()));
    ctx.request_repaint();
    Ok(mode)
}


