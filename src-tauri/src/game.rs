use crate::events::GameEvent;
use crate::models::{Account, Mode, Settings};
use crate::net::{sync_build, Manifest};
use crate::util::{format_error, read_json};
use mc_launcher_core::prelude::{
    Account as McAccount, CompatibilityPolicy, InstallRequest, JavaInstallPolicy, LaunchOptions,
    Launcher, LoaderSpec, LoaderVersion,
};
use mc_launcher_core::progress::{InstallStage, ProgressEvent};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::Emitter;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn status(on_event: &mut dyn FnMut(GameEvent), text: impl Into<String>) {
    on_event(GameEvent::Status(text.into()));
}

fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

fn hide_console(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[derive(Clone)]
struct JavaInstall {
    javaw: PathBuf,
    major: u32,
}

fn required_java_major(mc_version: &str) -> u32 {
    let mut parts = mc_version.split('.');
    let major: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    if major != 1 {
        return 21;
    }
    let minor: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(20);
    let patch: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    if minor <= 16 {
        8
    } else if minor < 20 || (minor == 20 && patch < 5) {
        17
    } else {
        21
    }
}

fn parse_java_major(output: &str) -> Option<u32> {
    let line = output.lines().find(|line| line.contains("version \""))?;
    let version = line.split("version \"").nth(1)?.split('"').next()?;
    let mut parts = version.split('.');
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn java_major(java: &Path) -> Option<u32> {
    let mut command = Command::new(java);
    command.arg("-version");
    hide_console(&mut command);
    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    parse_java_major(&text).or_else(|| parse_java_major(&String::from_utf8_lossy(&output.stdout)))
}

fn java_from_java_exe(java: PathBuf) -> Option<JavaInstall> {
    let major = java_major(&java)?;
    let javaw = java.with_file_name("javaw.exe");
    Some(JavaInstall {
        javaw: if javaw.exists() { javaw } else { java },
        major,
    })
}

fn java_candidate_dirs() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        roots.extend([
            PathBuf::from(r"C:\Program Files\Java"),
            PathBuf::from(r"C:\Program Files (x86)\Java"),
            PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
            PathBuf::from(r"C:\Program Files\Microsoft"),
            PathBuf::from(r"C:\Program Files\Zulu"),
            PathBuf::from(r"C:\Program Files\Amazon Corretto"),
            PathBuf::from(r"C:\Program Files\BellSoft"),
            PathBuf::from(r"C:\Program Files\Semeru"),
        ]);
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            roots.push(PathBuf::from(local).join("Programs").join("Eclipse Adoptium"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            roots.push(PathBuf::from(home).join(".jdks"));
        }
    }
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        roots.push(PathBuf::from(java_home));
    }
    if let Some(path) = std::env::var_os("PATH") {
        roots.extend(std::env::split_paths(&path));
    }

    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        let mut check = |path: PathBuf| {
            let key = path.to_string_lossy().to_lowercase();
            if path.is_file() && seen.insert(key) {
                candidates.push(path);
            }
        };
        check(root.join("bin").join("java.exe"));
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let home = entry.path();
                check(home.join("bin").join("java.exe"));
            }
        }
    }
    candidates
}

fn installed_javas() -> &'static Vec<JavaInstall> {
    static CACHE: OnceLock<Vec<JavaInstall>> = OnceLock::new();
    CACHE.get_or_init(|| {
        java_candidate_dirs()
            .into_iter()
            .filter_map(java_from_java_exe)
            .collect()
    })
}

fn select_java(settings: &Settings, mc_version: &str) -> Result<JavaInstall, String> {
    let required = required_java_major(mc_version);
    let custom = settings.java_path.trim();
    if !custom.is_empty() {
        let raw = PathBuf::from(custom);
        let java = if raw.is_dir() {
            raw.join("bin").join("java.exe")
        } else {
            raw
        };
        if java.exists() {
            if let Some(found) = java_from_java_exe(java.clone()) {
                if found.major == required || (required >= 17 && found.major > required) {
                    return Ok(found);
                }
            }
        }
    }

    let javas = installed_javas();
    if let Some(found) = javas.iter().find(|java| java.major == required) {
        return Ok(found.clone());
    }
    if required >= 17 {
        if let Some(found) = javas
            .iter()
            .filter(|java| java.major > required)
            .min_by_key(|java| java.major)
        {
            return Ok(found.clone());
        }
    }

    Err(format!(
        "Для Minecraft {mc_version} нужна Java {required}. Установи Java {required} или укажи путь к ней в настройках."
    ))
}

pub fn start(
    mode: Mode,
    account: Account,
    settings: Settings,
    install_only: bool,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let mode_id = mode.id.clone();
    let mut emit = |event: GameEvent| {
        let payload = crate::events::GameEventMessage {
            mode_id: mode_id.clone(),
            event,
        };
        let _ = app.emit("game:event", payload);
    };
    let result = run(
        &mode,
        &account,
        &settings,
        install_only,
        cancel.as_ref(),
        &mut emit,
    );
    if let Err(err) = &result {
        if cancelled(cancel.as_ref())
            || err.to_lowercase().contains("отмен")
            || err.to_lowercase().contains("abort")
        {
            emit(GameEvent::Cancelled);
        } else {
            emit(GameEvent::Error(err.clone()));
        }
    }
    result
}

fn run(
    mode: &Mode,
    account: &Account,
    settings: &Settings,
    install_only: bool,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(GameEvent),
) -> Result<(), String> {
    status(on_event, "Подготовка");
    status(on_event, "Подготовка");
    let java_choice = select_java(settings, &mode.mc_version)?;
    crate::logging::log(format!(
        "game: java={} mc={}",
        java_choice.javaw.display(),
        mode.mc_version
    ));
    let mut effective_settings = settings.clone();
    effective_settings.java_path = java_choice.javaw.to_string_lossy().to_string();
    let settings = &effective_settings;
    let game_dir = resolve_game_dir(mode, settings);
    fs::create_dir_all(&game_dir).map_err(|e| format!("Не удалось создать папку режима: {e}"))?;

    let has_build = mode.build_files() > 0;
    let loader = !matches!(mode.loader.as_str(), "" | "vanilla");

    if has_build {
        let manifest_path = game_dir.join(".manifest.json");
        let local: Option<Manifest> = read_json(&manifest_path);
        let needs_sync = install_only
            || local.is_none()
            || local
                .as_ref()
                .map(|m| !m.version.is_empty() && m.version != mode.version)
                .unwrap_or(true);
        if needs_sync {
            match sync_build(mode, settings, &game_dir, cancel, on_event) {
                Ok(()) => {}
                // Если сборка уже стоит, а сеть отвалилась — даём играть оффлайн.
                Err(err) if !install_only && local.is_some() => {
                    status(on_event, format!("Не удалось проверить обновления: {err}"));
                }
                Err(err) => return Err(err),
            }
        }
        if cancelled(cancel) {
            return Err("Операция отменена".to_string());
        }
        if install_only {
            if loader {
                verify_java(settings, on_event)?;
                if cancelled(cancel) {
                    return Err("Операция отменена".to_string());
                }
                install_profile(mode, settings, cancel, on_event)?;
            }
            on_event(GameEvent::Installed);
            return Ok(());
        }
    }

    if install_only {
        if loader {
            verify_java(settings, on_event)?;
        }
        if cancelled(cancel) {
            return Err("Операция отменена".to_string());
        }
        install_profile(mode, settings, cancel, on_event)?;
        on_event(GameEvent::Installed);
        return Ok(());
    }

    verify_java(settings, on_event)?;
    if cancelled(cancel) {
        return Err("Операция отменена".to_string());
    }

    let version_id = install_profile(mode, settings, cancel, on_event)?;
    if cancelled(cancel) {
        return Err("Операция отменена".to_string());
    }

    launch(account, settings, &version_id, &game_dir, cancel, on_event)
}

pub fn uninstall(settings: &Settings, mode_id: &str) -> Result<(), String> {
    if mode_id.is_empty()
        || mode_id == ".."
        || mode_id.contains('/')
        || mode_id.contains('\\')
    {
        return Err("Некорректный режим".to_string());
    }
    let base = Path::new(&settings.dir);
    for dir in [base.join("games").join(mode_id), base.join(mode_id)] {
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .map_err(|e| format!("Не удалось удалить {}: {e}", dir.display()))?;
        }
    }
    Ok(())
}

fn resolve_game_dir(mode: &Mode, settings: &Settings) -> PathBuf {
    let base = PathBuf::from(&settings.dir);
    let new_dir = base.join("games").join(&mode.id);
    let old_dir = base.join(&mode.id);
    if old_dir.exists() && !new_dir.exists() {
        if let Some(parent) = new_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::rename(&old_dir, &new_dir).is_err() {
            return old_dir;
        }
    }
    new_dir
}

fn java_executable(settings: &Settings) -> PathBuf {
    let raw = settings.java_path.trim();
    if raw.is_empty() {
        PathBuf::from("java")
    } else {
        PathBuf::from(raw)
    }
}

fn java_for_install(settings: &Settings) -> PathBuf {
    let path = java_executable(settings);
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.eq_ignore_ascii_case("javaw.exe"))
        .unwrap_or(false)
    {
        path.with_file_name("java.exe")
    } else {
        path
    }
}

fn with_java_on_path<T>(settings: &Settings, f: impl FnOnce() -> T) -> T {
    let custom = settings.java_path.trim();
    if custom.is_empty() {
        return f();
    }
    let java = java_for_install(settings);
    let Some(parent) = java.parent() else {
        return f();
    };
    let mut path_dir = parent.to_path_buf();

    // Forge/NeoForge installer запускается ядром как `java`. Подсовываем
    // javaw под именем java.exe, чтобы не открывалось окно консоли.
    #[cfg(windows)]
    {
        let javaw = java.with_file_name("javaw.exe");
        if javaw.exists() {
            let shim = Path::new(&settings.dir).join("runtime").join("java-shim");
            if fs::create_dir_all(&shim).is_ok() {
                let target = shim.join("java.exe");
                if fs::copy(&javaw, &target).is_ok() {
                    path_dir = shim;
                } else {
                    let _ = fs::remove_file(&target);
                    if fs::copy(&javaw, &target).is_ok() {
                        path_dir = shim;
                    }
                }
            }
        }
    }

    let old_path = std::env::var_os("PATH");
    let mut paths = vec![path_dir];
    if let Some(old) = &old_path {
        paths.extend(std::env::split_paths(old));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        std::env::set_var("PATH", joined);
    }
    let result = f();
    if let Some(old) = old_path {
        std::env::set_var("PATH", old);
    }
    result
}

fn verify_java(settings: &Settings, on_event: &mut dyn FnMut(GameEvent)) -> Result<(), String> {
    status(on_event, "Подготовка");
    let java = java_for_install(settings);
    let mut command = Command::new(&java);
    command
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console(&mut command);
    let mut child = command.spawn().map_err(|e| {
        format!(
            "Java не найдена: {}. Установи подходящую Java (8/17/21) или укажи путь в настройках",
            format_error(e)
        )
    })?;
    let started = Instant::now();
    let timeout = Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let mut err = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    use std::io::Read;
                    let _ = stderr.read_to_string(&mut err);
                }
                let first = err
                    .lines()
                    .next()
                    .unwrap_or("java -version завершилась с ошибкой");
                return Err(format!(
                    "Java не запускается: {first}. Установи подходящую Java (8/17/21) или укажи путь в настройках"
                ));
            }
            Ok(None) => {
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(
                        "Java не отвечает (таймаут 15 секунд). Проверь путь к Java в настройках"
                            .to_string(),
                    );
                }
                std::thread::sleep(Duration::from_millis(80));
            }
            Err(err) => {
                return Err(format!("Не удалось проверить Java: {err}"));
            }
        }
    }
}

fn expected_forge_id(mode: &Mode, settings: &Settings) -> Option<String> {
    let mc = mode.mc_version.trim();
    if let Some(lv) = mode
        .loader_version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let full = if lv.contains('-') {
            lv.to_string()
        } else {
            format!("{mc}-{lv}")
        };
        let forge = full.strip_prefix(&format!("{mc}-")).unwrap_or(&full);
        return Some(format!("{mc}-forge-{forge}"));
    }
    // Если версия не указана — ищем уже установленный Forge для этой MC.
    let versions = Path::new(&settings.dir).join("versions");
    let prefix = format!("{mc}-forge-");
    let mut found: Vec<String> = fs::read_dir(versions)
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(&prefix))
        .collect();
    found.sort();
    found.pop()
}

fn loader_spec(mode: &Mode) -> Option<LoaderSpec> {
    let mc = mode.mc_version.trim();
    let exact = mode
        .loader_version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    match mode.loader.as_str() {
        "fabric" => Some(LoaderSpec::Fabric {
            version: exact
                .map(|v| LoaderVersion::Exact(v.to_string()))
                .unwrap_or(LoaderVersion::LatestStable),
        }),
        "quilt" => Some(LoaderSpec::Quilt {
            version: exact
                .map(|v| LoaderVersion::Exact(v.to_string()))
                .unwrap_or(LoaderVersion::LatestStable),
        }),
        "forge" => Some(LoaderSpec::Forge {
            version: match exact {
                Some(v) => LoaderVersion::Exact(if v.contains('-') {
                    v.to_string()
                } else {
                    format!("{mc}-{v}")
                }),
                None => LoaderVersion::Latest,
            },
        }),
        "neoforge" => Some(LoaderSpec::NeoForge {
            version: exact
                .map(|v| LoaderVersion::Exact(v.to_string()))
                .unwrap_or(LoaderVersion::Latest),
        }),
        _ => None,
    }
}

fn ensure_launcher_profiles(settings: &Settings) -> Result<(), String> {
    let path = Path::new(&settings.dir).join("launcher_profiles.json");
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Не удалось создать папку Minecraft: {e}"))?;
    }
    let data = r#"{"profiles":{},"selectedProfile":"","clientToken":"","authenticationDatabase":{},"launcherVersion":{"name":"2.1.1","format":21}}"#;
    fs::write(&path, data)
        .map_err(|e| format!("Не удалось создать launcher_profiles.json: {e}"))?;
    Ok(())
}

fn install_profile(
    mode: &Mode,
    settings: &Settings,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(GameEvent),
) -> Result<String, String> {
    if mode.mc_version.trim().is_empty() {
        return Err("В режиме не указана версия Minecraft".to_string());
    }
    ensure_launcher_profiles(settings)?;
    let launcher = Launcher::new(settings.dir.clone());

    // Forge не переустанавливаем каждый запуск: если профиль уже есть,
    // достаточно проверить ванильные файлы.
    if mode.loader == "forge" {
        if let Some(id) = expected_forge_id(mode, settings) {
            let json = Path::new(&settings.dir)
                .join("versions")
                .join(&id)
                .join(format!("{id}.json"));
            if json.exists() {
                status(on_event, "Установка игры");
                let request = InstallRequest::vanilla(mode.mc_version.clone());
                let mut reporter = progress_reporter(on_event, cancel);
                with_java_on_path(settings, || {
                    launcher.install_with_progress(request, &mut reporter)
                })
                .map_err(|e| format_error(e))?;
                return Ok(id);
            }
        }
    }

    let request = InstallRequest {
        minecraft_version: mode.mc_version.clone(),
        loader: loader_spec(mode),
        java: JavaInstallPolicy::Auto,
    };
    status(on_event, "Установка игры");
    let mut reporter = progress_reporter(on_event, cancel);
    let result = with_java_on_path(settings, || {
        launcher.install_with_progress(request, &mut reporter)
    });
    if cancelled(cancel) {
        return Err("Операция отменена".to_string());
    }
    result
        .map(|r| r.version_id)
        .map_err(|e| format!("Не удалось установить игру: {}", format_error(e)))
}

fn progress_reporter<'a>(
    on_event: &'a mut dyn FnMut(GameEvent),
    cancel: &'a AtomicBool,
) -> impl FnMut(ProgressEvent) + 'a {
    let mut percent = 0u32;
    let mut base = 0u32;
    let mut span = 5u32;
    let mut done = 0u32;
    move |event: ProgressEvent| {
        if cancelled(cancel) {
            return;
        }
        match event {
            ProgressEvent::StageStarted { stage } => {
                let (next_base, next_span) = match stage {
                    InstallStage::ResolveVersion => (2, 6),
                    InstallStage::DownloadLibraries => (8, 32),
                    InstallStage::DownloadAssets => (40, 35),
                    InstallStage::InstallRuntime => (75, 4),
                    InstallStage::LoaderInstall => (79, 12),
                    InstallStage::ExtractNatives => (91, 4),
                    InstallStage::Verify => (95, 4),
                };
                base = next_base;
                span = next_span;
                done = 0;
                percent = percent.max(base);
                let label = match stage {
                    InstallStage::ExtractNatives => "Распаковка файлов",
                    InstallStage::Verify => "Проверка файлов",
                    _ => "Установка игры",
                };
                on_event(GameEvent::Status(label.to_string()));
                on_event(GameEvent::Progress {
                    task: percent as u64,
                    total: 100,
                });
            }
            ProgressEvent::TaskStarted { .. } => {}
            ProgressEvent::TaskSkipped { .. } | ProgressEvent::TaskFinished { .. } => {
                done += 1;
                let target = base as f64 + span as f64 * (done as f64 / (done as f64 + 4.0));
                percent = percent.max(target as u32).min(base + span);
                on_event(GameEvent::Progress {
                    task: percent as u64,
                    total: 100,
                });
            }
            ProgressEvent::BytesReceived { .. } => {}
        }
    }
}

const AUTHLIB_INJECTOR_URL: &str =
    "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar";

const PERFORMANCE_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:G1HeapWastePercent=5",
    "-XX:G1MixedGCCountTarget=4",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:G1RSetUpdatingPauseTimePercent=5",
    "-XX:SurvivorRatio=32",
    "-XX:+PerfDisableSharedMem",
    "-XX:MaxTenuringThreshold=1",
    "-Dusing.aikars.flags=https://mcflags.emc.gs",
    "-Daikars.new.flags=true",
    "-Dfile.encoding=UTF-8",
    "-Dlog4j2.formatMsgNoLookups=true",
];

fn ensure_authlib_injector(
    settings: &Settings,
    on_event: &mut dyn FnMut(GameEvent),
) -> Result<PathBuf, String> {
    let path = Path::new(&settings.dir).join("authlib-injector.jar");
    if path
        .metadata()
        .map(|meta| meta.len() > 100_000)
        .unwrap_or(false)
    {
        return Ok(path);
    }
    status(on_event, "Подготовка");
    let cancel = AtomicBool::new(false);
    crate::net::download_file(AUTHLIB_INJECTOR_URL, &path, None, &cancel, |_, _| {})?;
    Ok(path)
}

fn launch(
    account: &Account,
    settings: &Settings,
    version_id: &str,
    game_dir: &Path,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(GameEvent),
) -> Result<(), String> {
    status(on_event, "Запуск игры");
    let launcher = Launcher::new(settings.dir.clone());
    let version = launcher
        .load_version(version_id)
        .map_err(|e| format!("Не удалось прочитать профиль версии: {}", format_error(e)))?;

    let java = java_executable(settings);
    let access_token = if account.access_token.trim().is_empty() {
        "0".to_string()
    } else {
        account.access_token.clone()
    };
    let options = LaunchOptions {
        account: McAccount::Microsoft {
            username: account.name.clone(),
            uuid: account.uuid.clone(),
            access_token,
        },
        java_executable: Some(java),
        game_directory: Some(game_dir.to_path_buf()),
        launcher_name: "Lavunio".to_string(),
        launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        custom_resolution: if settings.width > 0 && settings.height > 0 {
            Some((settings.width, settings.height))
        } else {
            None
        },
        compatibility: CompatibilityPolicy::Auto,
        ..Default::default()
    };

    let mut launch = launcher
        .build_launch_command_from_version(&version, options)
        .map_err(|e| format!("Не удалось собрать команду запуска: {}", format_error(e)))?;

    launch.args.insert(0, format!("-Xmx{}G", settings.ram_gb()));
    launch.args.insert(0, "-Xms512M".to_string());
    if settings.jvm_flags {
        launch
            .args
            .splice(0..0, PERFORMANCE_FLAGS.iter().map(|flag| flag.to_string()));
    }
    if account.is_ely() {
        match ensure_authlib_injector(settings, on_event) {
            Ok(path) => {
                launch.args.insert(
                    0,
                    format!("-javaagent:{}=ely.by", path.to_string_lossy()),
                );
                launch.args.insert(0, "-Dauthlibinjector.side=client".to_string());
            }
            Err(err) => {
                status(
                    on_event,
                    format!("Скины Ely.by: не удалось загрузить authlib-injector ({err})"),
                );
            }
        }
    }
    if settings.fullscreen && !launch.args.iter().any(|a| a == "--fullscreen") {
        launch.args.push("--fullscreen".to_string());
    }

    if cancelled(cancel) {
        return Err("Операция отменена".to_string());
    }

    let mut command = Command::new(&launch.executable);
    command
        .args(&launch.args)
        .current_dir(&launch.working_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (key, value) in &launch.env {
        command.env(key, value);
    }
    hide_console(&mut command);

    let mut child = command
        .spawn()
        .map_err(|e| format!("Не удалось запустить Java: {}", format_error(e)))?;
    on_event(GameEvent::Launched);
    let play_started = Instant::now();
    let mut code = -1;
    loop {
        if cancelled(cancel) {
            let _ = child.kill();
            if let Ok(status) = child.wait() {
                code = status.code().unwrap_or(-1);
            }
            break;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                code = status.code().unwrap_or(-1);
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(150)),
            Err(e) => return Err(format!("Ошибка ожидания процесса игры: {e}")),
        }
    }
    on_event(GameEvent::Played(play_started.elapsed().as_secs()));
    on_event(GameEvent::Exit(code));
    Ok(())
}
