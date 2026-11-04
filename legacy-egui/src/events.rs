use crate::models::{Account, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub msg: String,
    pub kind: ToastKind,
    pub born: f64,
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    Status(String),
    Progress { task: u64, total: u64 },
    Installed,
    Launched,
    Exit(i32),
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub enum PublishEvent {
    Step(String),
    Progress { task: u64, total: u64 },
    Error(String),
    Done(Mode),
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ModesLoaded {
        modes: Vec<Mode>,
        refreshed: bool,
        error: Option<String>,
    },
    Game {
        mode_id: String,
        event: GameEvent,
    },
    Publish(PublishEvent),
    Image {
        key: String,
        data: Option<Vec<u8>>,
    },
    SessionChecked(Option<Account>),
    ElyLogin(Result<Account, String>),
}
