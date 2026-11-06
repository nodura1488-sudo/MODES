use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_loader() -> String {
    "vanilla".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryEntry {
    pub version: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub date: String,
    pub time: Option<String>,
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Mode {
    pub id: String,
    pub title: String,
    pub mc_version: String,
    pub version: String,
    pub release_date: Option<String>,
    pub version_id: Option<String>,
    #[serde(default = "default_loader")]
    pub loader: String,
    pub loader_version: Option<String>,
    pub size: Option<String>,
    pub players: Option<String>,
    pub accent: Option<String>,
    pub tags: Vec<String>,
    pub image_url: Option<String>,
    pub image: Option<String>,
    pub description: String,
    pub changelog: String,
    pub history: Vec<HistoryEntry>,
    pub build: Option<BuildInfo>,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            mc_version: String::new(),
            version: String::new(),
            release_date: None,
            version_id: None,
            loader: default_loader(),
            loader_version: None,
            size: None,
            players: None,
            accent: None,
            tags: Vec::new(),
            image_url: None,
            image: None,
            description: String::new(),
            changelog: String::new(),
            history: Vec::new(),
            build: None,
        }
    }
}

impl Mode {
    pub fn build_files(&self) -> u64 {
        self.build.as_ref().map(|b| b.files).unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BuildInfo {
    pub files: u64,
    pub bytes: u64,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub ram_gb: u32,
    pub dir: String,
    pub java_path: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub launcher_width: f32,
    pub launcher_height: f32,
    pub github_repo: String,
    pub github_token: String,
    pub jvm_flags: bool,
    pub ely_client_id: String,
    pub minimize_on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ram_gb: 4,
            dir: String::new(),
            java_path: String::new(),
            width: 925,
            height: 530,
            fullscreen: false,
            launcher_width: 1200.0,
            launcher_height: 740.0,
            github_repo: "tommy-whinston/TMLauncher".to_string(),
            github_token: String::new(),
            jvm_flags: true,
            ely_client_id: "tm-launcher".to_string(),
            minimize_on_launch: true,
        }
    }
}

impl Settings {
    pub fn with_default_dir(default_dir: &str) -> Self {
        let mut s = Self::default();
        s.dir = default_dir.to_string();
        s
    }

    pub fn ram_gb(&self) -> u32 {
        self.ram_gb.clamp(1, 64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub client_token: String,
    pub added_at: u64,
    pub session_error: bool,
    pub expires_at: u64,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: "offline".to_string(),
            name: String::new(),
            uuid: String::new(),
            access_token: "0".to_string(),
            client_token: String::new(),
            added_at: 0,
            session_error: false,
            expires_at: 0,
        }
    }
}

impl Account {
    pub fn is_ely(&self) -> bool {
        self.kind == "ely"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeState {
    pub installed: bool,
    pub version: Option<String>,
    pub installed_at: u64,
    pub hidden: bool,
    pub library: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteCache {
    pub fetched_at: u64,
    pub modes: Vec<Mode>,
}

pub type ModeStates = HashMap<String, ModeState>;

#[derive(Debug, Clone, Deserialize)]
pub struct ElyProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElyAuthResponse {
    pub access_token: String,
    pub client_token: Option<String>,
    pub selected_profile: Option<ElyProfile>,
}
