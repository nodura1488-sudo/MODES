use crate::models::{Account, Mode, ModeState, ModeStates, RemoteCache, Settings};
use crate::util::{now_ms, read_json, write_json};
use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("TDEV Launcher")
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
}

impl Store {
    pub fn new() -> Self {
        let dir = app_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let default_dir = default_game_dir().to_string_lossy().to_string();

        let mut settings: Settings = read_json(&dir.join("settings.json"))
            .unwrap_or_else(|| Settings::with_default_dir(&default_dir));
        if settings.dir.trim().is_empty() {
            settings.dir = default_dir.clone();
        }
        if settings.github_repo.trim().is_empty() {
            settings.github_repo = "nodura1488-sudo/MODES".to_string();
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
        let state: ModeStates = read_json(&dir.join("mode-state.json")).unwrap_or_default();
        let client_token: Option<String> = read_json(&dir.join("client-token.json"));
        let client_token = client_token.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
            let token = uuid::Uuid::new_v4().to_string();
            let _ = write_json(&dir.join("client-token.json"), &token);
            token
        });
        let current_account_id: Option<String> = read_json(&dir.join("current-account.json"));

        Self {
            dir,
            settings,
            accounts,
            state,
            client_token,
            current_account_id,
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

    pub fn upsert_account(&mut self, mut acc: Account) -> Result<(), String> {
        if let Some(existing) = self
            .accounts
            .iter_mut()
            .find(|a| a.kind == acc.kind && a.name.eq_ignore_ascii_case(&acc.name))
        {
            acc.id = existing.id.clone();
            *existing = acc;
        } else {
            self.accounts.push(acc);
        }
        self.save_accounts()
    }

    pub fn remove_account(&mut self, id: &str) -> Result<(), String> {
        self.accounts.retain(|a| a.id != id);
        self.save_accounts()?;
        if self.current_account_id.as_deref() == Some(id) {
            let next = self.accounts.first().map(|a| a.id.clone());
            self.set_current_account(next)?;
        }
        Ok(())
    }

    pub fn set_mode_installed(&mut self, mode_id: &str, version: &str) -> Result<(), String> {
        self.state.insert(
            mode_id.to_string(),
            ModeState {
                installed: true,
                version: Some(version.to_string()),
                installed_at: now_ms(),
            },
        );
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
