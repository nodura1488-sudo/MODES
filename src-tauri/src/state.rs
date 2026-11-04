use crate::models::Mode;
use crate::store::Store;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub store: Mutex<Store>,
    pub modes: Mutex<Vec<Mode>>,
    pub active_game: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Store::new()),
            modes: Mutex::new(Vec::new()),
            active_game: Arc::new(Mutex::new(None)),
        }
    }
}
