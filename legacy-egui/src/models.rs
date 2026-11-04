use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_loader() -> String {
    "vanilla".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Mode {
    pub id: String,
    pub title: String,
    pub mc_version: String,
    pub version: String,
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
    pub build: Option<BuildInfo>,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            mc_version: String::new(),
            version: String::new(),
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
            build: None,
        }
    }
}

impl Mode {
    pub fn loader_label(&self) -> &'static str {
        match self.loader.as_str() {
            "forge" => "Forge",
            "fabric" => "Fabric",
            "quilt" => "Quilt",
            "neoforge" => "NeoForge",
            _ => "Vanilla",
        }
    }

    pub fn loader_title(&self) -> String {
        if self.loader == "vanilla" || self.loader.is_empty() {
            "Vanilla".to_string()
        } else if let Some(v) = self.loader_version.as_deref().filter(|v| !v.is_empty()) {
            format!("{} {}", self.loader_label(), v)
        } else {
            self.loader_label().to_string()
        }
    }

    pub fn build_files(&self) -> u64 {
        self.build.as_ref().map(|b| b.files).unwrap_or(0)
    }

    pub fn image_source(&self) -> Option<&str> {
        self.image_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.image.as_deref().filter(|s| !s.is_empty()))
    }

    pub fn accent_rgb(&self) -> (u8, u8, u8) {
        parse_hex_color(self.accent.as_deref().unwrap_or("#8b5cf6"))
            .unwrap_or((139, 92, 246))
    }
}

pub fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let s = value.trim().trim_start_matches('#');
    if s.len() == 3 {
        let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
        return Some((r, g, b));
    }
    if s.len() != 6 {
        return None;
    }
    Some((
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ))
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
            github_repo: "nodura1488-sudo/MODES".to_string(),
            github_token: String::new(),
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
        }
    }
}

impl Account {
    pub fn is_ely(&self) -> bool {
        self.kind == "ely"
    }

    pub fn initial(&self) -> String {
        self.name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string())
    }

    pub fn skin_url(&self) -> Option<String> {
        if self.is_ely() && !self.name.is_empty() {
            Some(format!(
                "https://skinsystem.ely.by/skins/{}.png",
                urlencoding_name(&self.name)
            ))
        } else {
            None
        }
    }
}

fn urlencoding_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c.to_string()
            } else {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf)
                    .as_bytes()
                    .iter()
                    .map(|b| format!("%{b:02X}"))
                    .collect::<String>()
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeState {
    pub installed: bool,
    pub version: Option<String>,
    pub installed_at: u64,
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
    pub available_profiles: Option<Vec<ElyProfile>>,
}


