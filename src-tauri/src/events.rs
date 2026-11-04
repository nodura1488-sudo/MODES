use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum GameEvent {
    Status(String),
    Progress { task: u64, total: u64 },
    Installed,
    Launched,
    Exit(i32),
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameEventMessage {
    #[serde(rename = "modeId")]
    pub mode_id: String,
    #[serde(flatten)]
    pub event: GameEvent,
}
