use crate::models::{ElyAuthResponse, ElyProfile};
use crate::util::format_error;
use md5::{Digest, Md5};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;

const BASE: &str = "https://authserver.ely.by/auth";

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(25))
            .user_agent("TDEV-Launcher/0.2 (Rust)")
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn error_message(status: reqwest::StatusCode, data: &Value) -> String {
    data.get("errorMessage")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            data.get("error")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Ely.by ответил ошибкой ({status})"))
        })
}

pub fn ely_login(username: &str, password: &str, client_token: &str) -> Result<ElyAuthResponse, String> {
    let body = json!({
        "agent": { "name": "Minecraft", "version": 1 },
        "username": username,
        "password": password,
        "clientToken": client_token,
        "requestUser": true
    });
    let res = client()
        .post(format!("{BASE}/authenticate"))
        .json(&body)
        .send()
        .map_err(|e| format!("Нет связи с Ely.by: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(error_message(status, &data));
    }
    serde_json::from_value::<ElyAuthResponse>(data)
        .map_err(|_| "Ely.by вернул неожиданный ответ".to_string())
}

pub fn ely_validate(access_token: &str, client_token: &str) -> bool {
    let body = json!({
        "accessToken": access_token,
        "clientToken": client_token
    });
    client()
        .post(format!("{BASE}/validate"))
        .json(&body)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub fn ely_refresh(access_token: &str, client_token: &str) -> Result<ElyAuthResponse, String> {
    let body = json!({
        "accessToken": access_token,
        "clientToken": client_token,
        "requestUser": true
    });
    let res = client()
        .post(format!("{BASE}/refresh"))
        .json(&body)
        .send()
        .map_err(|e| format!("Нет связи с Ely.by: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(error_message(status, &data));
    }
    serde_json::from_value::<ElyAuthResponse>(data)
        .map_err(|_| "Ely.by вернул неожиданный ответ".to_string())
}

pub fn pick_profile(data: &ElyAuthResponse, fallback_name: &str) -> ElyProfile {
    data.selected_profile
        .clone()
        .or_else(|| data.available_profiles.as_ref().and_then(|p| p.first().cloned()))
        .unwrap_or_else(|| ElyProfile {
            id: offline_uuid(fallback_name),
            name: fallback_name.to_string(),
        })
}

/// UUID офлайн-аккаунта Minecraft: md5("OfflinePlayer:<ник>") с version=3 и variant=10xx.
pub fn offline_uuid(name: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{name}").as_bytes());
    let mut hash: [u8; 16] = hasher.finalize().into();
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    let hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn valid_offline_name(name: &str) -> bool {
    let len = name.chars().count();
    if !(3..=16).contains(&len) {
        return false;
    }
    name.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || c == '_'
            || c == '-'
            || ('А'..='я').contains(&c)
            || c == 'Ё'
            || c == 'ё'
    })
}
