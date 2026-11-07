use crate::models::ElyAuthResponse;
use crate::util::format_error;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;

const BASE: &str = "https://authserver.ely.by/auth";
const OAUTH_BASE: &str = "https://account.ely.by/api/oauth2/v1";
const OAUTH_INFO: &str = "https://account.ely.by/api/account/v1/info";
const OAUTH_SCOPES: &str = "account_info offline_access minecraft_server_session";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub verification_uri_complete: String,
    pub interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ElyUserInfo {
    pub uuid: String,
    pub username: String,
}

pub enum DevicePoll {
    Pending,
    SlowDown,
    Token(OAuthToken),
}

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(25))
            .user_agent("Lavunio/1.0 (Rust)")
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

fn oauth_error(status: reqwest::StatusCode, data: &Value) -> String {
    data.get("error_description")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            data.get("message")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            data.get("error")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .map(str::to_string)
        .unwrap_or_else(|| format!("Ely.by ответил ошибкой ({status})"))
}

pub fn device_start(client_id: &str) -> Result<DeviceCode, String> {
    let res = client()
        .post(format!("{OAUTH_BASE}/devicecode"))
        .form(&[("client_id", client_id), ("scope", OAUTH_SCOPES)])
        .send()
        .map_err(|e| format!("Нет связи с Ely.by: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(oauth_error(status, &data));
    }
    serde_json::from_value::<DeviceCode>(data)
        .map_err(|_| "Ely.by вернул неожиданный ответ".to_string())
}

pub fn device_poll(client_id: &str, device_code: &str) -> Result<DevicePoll, String> {
    let res = client()
        .post(format!("{OAUTH_BASE}/token"))
        .form(&[
            ("client_id", client_id),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .map_err(|e| format!("Нет связи с Ely.by: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| json!({}));
    if status.is_success() {
        let token = serde_json::from_value::<OAuthToken>(data)
            .map_err(|_| "Ely.by вернул неожиданный ответ".to_string())?;
        return Ok(DevicePoll::Token(token));
    }
    match data.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => Ok(DevicePoll::Pending),
        Some("slow_down") => Ok(DevicePoll::SlowDown),
        Some("access_denied") => Err("Вход отменён на сайте Ely.by".to_string()),
        Some("expired_token") => Err("Код входа истёк — попробуй ещё раз".to_string()),
        _ => Err(oauth_error(status, &data)),
    }
}

pub fn oauth_refresh(client_id: &str, refresh_token: &str) -> Result<OAuthToken, String> {
    let res = client()
        .post(format!("{OAUTH_BASE}/token"))
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .map_err(|e| format!("Нет связи с Ely.by: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(oauth_error(status, &data));
    }
    serde_json::from_value::<OAuthToken>(data)
        .map_err(|_| "Ely.by вернул неожиданный ответ".to_string())
}

pub fn user_info(access_token: &str) -> Result<ElyUserInfo, String> {
    let res = client()
        .get(OAUTH_INFO)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| format!("Нет связи с Ely.by: {}", format_error(e)))?;
    let status = res.status();
    let data: Value = res.json().unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(oauth_error(status, &data));
    }
    serde_json::from_value::<ElyUserInfo>(data)
        .map_err(|_| "Ely.by вернул неожиданный профиль".to_string())
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
