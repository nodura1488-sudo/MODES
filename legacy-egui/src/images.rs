use crate::events::AppEvent;
use base64::Engine as _;
use eframe::egui;
use egui::{ColorImage, Rect, TextureHandle, TextureOptions, Vec2};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub struct ImageStore {
    textures: HashMap<String, TextureHandle>,
    loading: HashSet<String>,
    failed: HashSet<String>,
    tx: Sender<AppEvent>,
}

impl ImageStore {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Self {
            textures: HashMap::new(),
            loading: HashSet::new(),
            failed: HashSet::new(),
            tx,
        }
    }

    pub fn texture(&self, key: &str) -> Option<TextureHandle> {
        self.textures.get(key).cloned()
    }

    pub fn ensure(&mut self, key: &str, source: &str, ctx: &egui::Context) {
        if key.is_empty()
            || self.textures.contains_key(key)
            || self.loading.contains(key)
            || self.failed.contains(key)
        {
            return;
        }
        if source.starts_with("data:image/") {
            if let Some(bytes) = decode_data_url(source) {
                self.load_bytes(key, &bytes, ctx);
            } else {
                self.failed.insert(key.to_string());
            }
            return;
        }
        self.loading.insert(key.to_string());
        let key_owned = key.to_string();
        let source_owned = source.to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let data = fetch_image(&source_owned);
            let _ = tx.send(AppEvent::Image {
                key: key_owned,
                data,
            });
        });
    }

    pub fn handle(&mut self, key: String, data: Option<Vec<u8>>, ctx: &egui::Context) {
        self.loading.remove(&key);
        match data {
            Some(bytes) => {
                if !self.load_bytes(&key, &bytes, ctx) {
                    self.failed.insert(key);
                }
            }
            None => {
                self.failed.insert(key);
            }
        }
    }

    fn load_bytes(&mut self, key: &str, bytes: &[u8], ctx: &egui::Context) -> bool {
        let Ok(image) = image::load_from_memory(bytes) else {
            return false;
        };
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let texture = ctx.load_texture(key.to_string(), color, TextureOptions::LINEAR);
        self.textures.insert(key.to_string(), texture);
        true
    }

    pub fn load_local(&mut self, key: &str, path: &Path, ctx: &egui::Context) -> bool {
        if self.textures.contains_key(key) {
            return true;
        }
        match fs::read(path) {
            Ok(bytes) => {
                let ok = self.load_bytes(key, &bytes, ctx);
                if !ok {
                    self.failed.insert(key.to_string());
                }
                ok
            }
            Err(_) => {
                self.failed.insert(key.to_string());
                false
            }
        }
    }
}

fn decode_data_url(source: &str) -> Option<Vec<u8>> {
    let (_, data) = source.split_once(',')?;
    base64::engine::general_purpose::STANDARD.decode(data).ok()
}

fn fetch_image(url: &str) -> Option<Vec<u8>> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("TDEV-Launcher/0.2 (Rust)")
            .build()
            .ok()?;
        let res = client.get(url).send().ok()?;
        if !res.status().is_success() {
            return None;
        }
        res.bytes().ok().map(|b| b.to_vec())
    } else {
        fs::read(url).ok()
    }
}

/// UV-обрезка «cover»: картинка заполняет прямоугольник без искажений.
pub fn cover_uv(image: Vec2, target: Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    }
    let image_aspect = image.x / image.y;
    let target_aspect = target.x / target.y;
    if image_aspect > target_aspect {
        let w = target_aspect / image_aspect;
        let x = (1.0 - w) / 2.0;
        Rect::from_min_max(egui::pos2(x, 0.0), egui::pos2(x + w, 1.0))
    } else {
        let h = image_aspect / target_aspect;
        let y = (1.0 - h) / 2.0;
        Rect::from_min_max(egui::pos2(0.0, y), egui::pos2(1.0, y + h))
    }
}

pub fn face_uv() -> Rect {
    Rect::from_min_max(egui::pos2(8.0 / 64.0, 8.0 / 64.0), egui::pos2(16.0 / 64.0, 16.0 / 64.0))
}
