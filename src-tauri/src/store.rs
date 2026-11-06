use crate::models::{Account, Mode, ModeState, ModeStates, RemoteCache, Settings};
use crate::util::{now_ms, read_json, write_json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("TMStore")
}

/// Переносит настройки и аккаунты из папки старого имени при первом запуске.
fn migrate_legacy_data(new_dir: &Path) {
    if new_dir.join("settings.json").exists() {
        return;
    }
    let Some(base) = dirs::data_dir() else {
        return;
    };
    for old_name in ["TMLauncher", "TM Launcher", "TDEV Launcher"] {
        let old_dir = base.join(old_name);
        if !old_dir.is_dir() {
            continue;
        }
        let has_data = ["settings.json", "accounts.json", "mode-state.json"]
            .iter()
            .any(|name| old_dir.join(name).exists());
        if !has_data {
            continue;
        }
        let Ok(entries) = fs::read_dir(&old_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name() {
                    let dest = new_dir.join(name);
                    if !dest.exists() {
                        let _ = fs::copy(&path, dest);
                    }
                }
            }
        }
        break;
    }
}

pub fn default_game_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tdevlauncher")
}

pub struct Store {
    pub dir: PathBuf,
    pub settings: Settings,
    pub accounts: Vec<Account>,
    pub state: ModeStates,
    pub client_token: String,
    pub current_account_id: Option<String>,
    pub refresh_tokens: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        let dir = app_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        migrate_legacy_data(&dir);
        let default_dir = default_game_dir().to_string_lossy().to_string();

        let mut settings: Settings = read_json(&dir.join("settings.json"))
            .unwrap_or_else(|| Settings::with_default_dir(&default_dir));
        if settings.dir.trim().is_empty() {
            settings.dir = default_dir.clone();
        }
        if settings.github_repo.trim().is_empty()
            || settings.github_repo == "tommy-whinston/TMLauncher"
        {
            settings.github_repo = "tommy-whinston/TMStore".to_string();
        }
        if settings.ely_client_id.trim().is_empty() {
            settings.ely_client_id = "tm-launcher".to_string();
        }
        if settings.launcher_width < 980.0 {
            settings.launcher_width = 1200.0;
        }
        if settings.launcher_height < 640.0 {
            settings.launcher_height = 740.0;
        }
        if settings.ram_gb == 0 {
            settings.ram_gb = 4;
        }
        if settings.width == 0 {
            settings.width = 925;
        }
        if settings.height == 0 {
            settings.height = 530;
        }

        let accounts: Vec<Account> = read_json(&dir.join("accounts.json")).unwrap_or_default();
        let mut state: ModeStates = read_json(&dir.join("mode-state.json")).unwrap_or_default();
        let mut migrated = false;
        for mode_state in state.values_mut() {
            if mode_state.library.is_none() {
                mode_state.library = Some(mode_state.installed && !mode_state.hidden);
                migrated = true;
            }
        }
        if migrated {
            let _ = write_json(&dir.join("mode-state.json"), &state);
        }
        let client_token: Option<String> = read_json(&dir.join("client-token.json"));
        let client_token = client_token
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                let token = uuid::Uuid::new_v4().to_string();
                let _ = write_json(&dir.join("client-token.json"), &token);
                token
            });
        let current_account_id: Option<String> = read_json(&dir.join("current-account.json"));
        let refresh_tokens: HashMap<String, String> =
            read_json(&dir.join("refresh-tokens.json")).unwrap_or_default();

        Self {
            dir,
            settings,
            accounts,
            state,
            client_token,
            current_account_id,
            refresh_tokens,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn save_settings(&self) -> Result<(), String> {
        write_json(&self.path("settings.json"), &self.settings)
    }

    pub fn save_accounts(&self) -> Result<(), String> {
        write_json(&self.path("accounts.json"), &self.accounts)
    }

    pub fn save_refresh_tokens(&self) -> Result<(), String> {
        write_json(&self.path("refresh-tokens.json"), &self.refresh_tokens)
    }

    pub fn save_state(&self) -> Result<(), String> {
        write_json(&self.path("mode-state.json"), &self.state)
    }

    pub fn save_current_account(&self) -> Result<(), String> {
        write_json(&self.path("current-account.json"), &self.current_account_id)
    }

    pub fn get_current_account(&self) -> Option<&Account> {
        let id = self.current_account_id.as_deref()?;
        self.accounts.iter().find(|a| a.id == id)
    }

    pub fn current_account(&self) -> Option<Account> {
        self.get_current_account().cloned()
    }

    pub fn set_current_account(&mut self, id: Option<String>) -> Result<(), String> {
        self.current_account_id = id;
        self.save_current_account()
    }

    pub fn upsert_account(&mut self, mut acc: Account) -> Result<Account, String> {
        if let Some(existing) = self
            .accounts
            .iter_mut()
            .find(|a| a.kind == acc.kind && a.name.eq_ignore_ascii_case(&acc.name))
        {
            acc.id = existing.id.clone();
            *existing = acc.clone();
        } else {
            self.accounts.push(acc.clone());
        }
        self.save_accounts()?;
        Ok(acc)
    }

    pub fn remove_account(&mut self, id: &str) -> Result<(), String> {
        self.accounts.retain(|a| a.id != id);
        self.refresh_tokens.remove(id);
        self.save_accounts()?;
        let _ = self.save_refresh_tokens();
        if self.current_account_id.as_deref() == Some(id) {
            let next = self.accounts.first().map(|a| a.id.clone());
            self.set_current_account(next)?;
        }
        Ok(())
    }

    pub fn set_mode_installed(&mut self, mode_id: &str, version: &str) -> Result<(), String> {
        let playtime_secs = self
            .state
            .get(mode_id)
            .map(|state| state.playtime_secs)
            .unwrap_or(0);
        self.state.insert(
            mode_id.to_string(),
            ModeState {
                installed: true,
                version: Some(version.to_string()),
                installed_at: now_ms(),
                hidden: false,
                library: Some(true),
                playtime_secs,
            },
        );
        self.save_state()
    }

    pub fn set_mode_library(&mut self, mode_id: &str, library: bool) -> Result<(), String> {
        let state = self
            .state
            .entry(mode_id.to_string())
            .or_insert_with(ModeState::default);
        state.library = Some(library);
        state.hidden = !library;
        self.save_state()
    }

    pub fn remote_cache_path(&self) -> PathBuf {
        self.path("remote-modes.json")
    }

    pub fn read_cached_modes(&self) -> Vec<Mode> {
        read_json::<RemoteCache>(&self.remote_cache_path())
            .map(|c| c.modes)
            .unwrap_or_default()
    }

    pub fn cache_modes(&self, modes: &[Mode]) -> Result<(), String> {
        write_json(
            &self.remote_cache_path(),
            &RemoteCache {
                fetched_at: now_ms(),
                modes: modes.to_vec(),
            },
        )
    }
}
