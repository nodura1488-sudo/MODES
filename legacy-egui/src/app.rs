use crate::auth;
use crate::events::{AppEvent, GameEvent, PublishEvent, Toast, ToastKind};
use crate::game;
use crate::images::{cover_uv, ImageStore};
use crate::models::{parse_hex_color, Account, Mode, ModeState, Settings};
use crate::net;
use crate::publish::{self, PublishForm};
use crate::store::Store;
use crate::theme;
use crate::util::{human_size, now_ms, open_path, open_url, path_string, shorten};
use crate::widgets::{self, Icon};
use eframe::egui;
use egui::{
    pos2, vec2, Align, Align2, Color32, Frame, Id, Layout, Margin, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2, ViewportCommand,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

const SIDEBAR_WIDTH: f32 = 348.0;
const TITLEBAR_HEIGHT: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalKind {
    Settings,
    Account,
    Publish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccountTab {
    Ely,
    Offline,
}

#[derive(Debug, Clone)]
struct SettingsForm {
    ram_gb: u32,
    dir: String,
    java_path: String,
    width: String,
    height: String,
    fullscreen: bool,
}

impl SettingsForm {
    fn from_settings(s: &Settings) -> Self {
        Self {
            ram_gb: s.ram_gb(),
            dir: s.dir.clone(),
            java_path: s.java_path.clone(),
            width: s.width.to_string(),
            height: s.height.to_string(),
            fullscreen: s.fullscreen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Install,
    Update,
    Play,
    Running,
}

struct Busy {
    mode_id: String,
}

pub struct App {
    store: Store,
    modes: Vec<Mode>,
    selected: Option<String>,
    images: ImageStore,
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    toast: Option<Toast>,
    modal: Option<ModalKind>,
    settings_form: SettingsForm,
    account_tab: AccountTab,
    login_user: String,
    login_pass: String,
    login_error: Option<String>,
    login_busy: bool,
    offline_name: String,
    offline_error: Option<String>,
    publish_form: PublishForm,
    publish_error: Option<String>,
    publish_step: String,
    publish_progress: f32,
    publish_busy: bool,
    pub_art_name: String,
    pub_zip_name: String,
    busy: Option<Busy>,
    running: Option<String>,
    progress: f32,
    status: String,
    cancel: Option<Arc<AtomicBool>>,
    refreshing: bool,
    time: f64,
    last_size_save: f64,
    last_inner_size: Vec2,
    session_checked: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, store: Store) -> Self {
        theme::install(&cc.egui_ctx);
        let (tx, rx) = channel();
        let cached = store.read_cached_modes();
        let selected = cached.first().map(|m| m.id.clone());
        let settings_form = SettingsForm::from_settings(&store.settings);
        let mut publish_form = PublishForm {
            repo: store.settings.github_repo.clone(),
            token: store.settings.github_token.clone(),
            version: "1.0.0".to_string(),
            mc_version: "1.20.1".to_string(),
            loader: "vanilla".to_string(),
            accent: "#8b5cf6".to_string(),
            ..Default::default()
        };
        publish_form.tags = Vec::new();
        let last_inner_size = vec2(
            store.settings.launcher_width,
            store.settings.launcher_height,
        );

        let mut app = Self {
            store,
            modes: cached,
            selected,
            images: ImageStore::new(tx.clone()),
            tx,
            rx,
            toast: None,
            modal: None,
            settings_form,
            account_tab: AccountTab::Ely,
            login_user: String::new(),
            login_pass: String::new(),
            login_error: None,
            login_busy: false,
            offline_name: String::new(),
            offline_error: None,
            publish_form,
            publish_error: None,
            publish_step: String::new(),
            publish_progress: 0.0,
            publish_busy: false,
            pub_art_name: String::new(),
            pub_zip_name: String::new(),
            busy: None,
            running: None,
            progress: 0.0,
            status: "Готов к запуску".to_string(),
            cancel: None,
            refreshing: false,
            time: 0.0,
            last_size_save: 0.0,
            last_inner_size,
            session_checked: false,
        };
        app.spawn_modes_load(&cc.egui_ctx, false);
        app.spawn_session_check(&cc.egui_ctx);
        app
    }

    fn spawn_modes_load(&self, ctx: &egui::Context, refreshed: bool) {
        let repo = self.store.settings.github_repo.clone();
        if repo.trim().is_empty() {
            return;
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let event = match net::fetch_remote_modes(&repo) {
                Ok(modes) => AppEvent::ModesLoaded {
                    modes,
                    refreshed,
                    error: None,
                },
                Err(err) => AppEvent::ModesLoaded {
                    modes: Vec::new(),
                    refreshed,
                    error: Some(err),
                },
            };
            let _ = tx.send(event);
            ctx.request_repaint();
        });
    }

    fn spawn_session_check(&mut self, ctx: &egui::Context) {
        if self.session_checked {
            return;
        }
        self.session_checked = true;
        let Some(acc) = self.store.current_account() else {
            return;
        };
        if !acc.is_ely() {
            return;
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut acc = acc;
            if auth::ely_validate(&acc.access_token, &acc.client_token) {
                let _ = tx.send(AppEvent::SessionChecked(Some(acc)));
            } else {
                match auth::ely_refresh(&acc.access_token, &acc.client_token) {
                    Ok(data) => {
                        acc.access_token = data.access_token;
                        if let Some(token) = data.client_token {
                            acc.client_token = token;
                        }
                        if let Some(profile) = data.selected_profile {
                            acc.name = profile.name;
                            acc.uuid = profile.id;
                        }
                        acc.session_error = false;
                        let _ = tx.send(AppEvent::SessionChecked(Some(acc)));
                    }
                    Err(_) => {
                        acc.session_error = true;
                        let _ = tx.send(AppEvent::SessionChecked(Some(acc)));
                    }
                }
            }
            ctx.request_repaint();
        });
    }

    fn poll_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::ModesLoaded {
                    modes,
                    refreshed,
                    error,
                } => {
                    self.refreshing = false;
                    if let Some(err) = error {
                        if refreshed {
                            self.toast(format!("Не удалось обновить список: {err}"), ToastKind::Warn);
                        }
                    } else {
                        self.modes = modes;
                        let _ = self.store.cache_modes(&self.modes);
                        if self.selected.is_none() {
                            self.selected = self.modes.first().map(|m| m.id.clone());
                        } else if !self.modes.iter().any(|m| Some(&m.id) == self.selected.as_ref()) {
                            self.selected = self.modes.first().map(|m| m.id.clone());
                        }
                        if refreshed {
                            self.toast("Список режимов обновлён", ToastKind::Ok);
                        }
                    }
                }
                AppEvent::Game { mode_id, event } => self.handle_game_event(ctx, &mode_id, event),
                AppEvent::Publish(event) => self.handle_publish_event(ctx, event),
                AppEvent::Image { key, data } => self.images.handle(key, data, ctx),
                AppEvent::SessionChecked(Some(acc)) => {
                    if let Some(existing) = self
                        .store
                        .accounts
                        .iter_mut()
                        .find(|a| a.id == acc.id)
                    {
                        let changed = existing.access_token != acc.access_token
                            || existing.name != acc.name
                            || existing.session_error != acc.session_error;
                        *existing = acc.clone();
                        if changed {
                            let _ = self.store.save_accounts();
                        }
                    }
                    if acc.session_error {
                        self.toast(
                            "Сессия Ely.by истекла — войдите заново",
                            ToastKind::Warn,
                        );
                    }
                }
                AppEvent::SessionChecked(None) => {}
                AppEvent::ElyLogin(result) => {
                    self.login_busy = false;
                    match result {
                        Ok(account) => {
                            let name = account.name.clone();
                            let _ = self.store.upsert_account(account.clone());
                            let _ = self.store.set_current_account(Some(account.id.clone()));
                            self.login_pass.clear();
                            self.login_error = None;
                            self.toast(format!("Добро пожаловать, {name}!"), ToastKind::Ok);
                        }
                        Err(err) => {
                            self.login_error = Some(err.clone());
                            self.toast(err, ToastKind::Error);
                        }
                    }
                }
            }
        }
    }

    fn handle_game_event(&mut self, ctx: &egui::Context, mode_id: &str, event: GameEvent) {
        let is_current = self.selected.as_deref() == Some(mode_id);
        match event {
            GameEvent::Status(text) => {
                if is_current {
                    self.status = text;
                }
            }
            GameEvent::Progress { task, total } => {
                let p = if total > 0 {
                    (task as f32 / total as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if is_current {
                    self.progress = p;
                    self.status = format!("Загрузка… {}%", (p * 100.0).round() as u32);
                }
            }
            GameEvent::Installed => {
                self.busy = None;
                self.cancel = None;
                if let Some(mode) = self.modes.iter().find(|m| m.id == mode_id) {
                    let _ = self.store.set_mode_installed(mode_id, &mode.version);
                }
                if is_current {
                    self.progress = 1.0;
                    self.status = "Установлено ✔".to_string();
                }
                self.toast("Режим установлен", ToastKind::Ok);
            }
            GameEvent::Launched => {
                self.busy = None;
                self.cancel = None;
                self.running = Some(mode_id.to_string());
                if is_current {
                    self.progress = 1.0;
                    self.status = "Игра запущена".to_string();
                }
                self.toast("Игра запущена. Удачной катки!", ToastKind::Ok);
            }
            GameEvent::Exit(code) => {
                self.running = None;
                self.busy = None;
                self.cancel = None;
                if is_current {
                    self.status = if code == 0 {
                        "Игра закрыта".to_string()
                    } else {
                        format!("Игра завершилась (код {code})")
                    };
                }
            }
            GameEvent::Error(err) => {
                self.busy = None;
                self.cancel = None;
                if is_current {
                    self.status = "Ошибка".to_string();
                }
                self.toast(err, ToastKind::Error);
            }
            GameEvent::Cancelled => {
                self.busy = None;
                self.cancel = None;
                if is_current {
                    self.status = "Отменено".to_string();
                }
                self.toast("Операция отменена", ToastKind::Warn);
            }
        }
        ctx.request_repaint();
    }

    fn handle_publish_event(&mut self, ctx: &egui::Context, event: PublishEvent) {
        match event {
            PublishEvent::Step(text) => {
                self.publish_step = text;
                self.publish_error = None;
            }
            PublishEvent::Progress { task, total } => {
                let p = if total > 0 {
                    task as f32 / total as f32
                } else {
                    0.0
                };
                self.publish_progress = p.clamp(0.0, 1.0);
                self.publish_step = format!("Загрузка файлов… {}%", (p * 100.0).round() as u32);
            }
            PublishEvent::Error(err) => {
                self.publish_busy = false;
                self.publish_error = Some(err.clone());
                self.publish_step = "Ошибка".to_string();
                self.toast(err, ToastKind::Error);
            }
            PublishEvent::Done(mode) => {
                self.publish_busy = false;
                self.publish_progress = 1.0;
                self.publish_step = "Готово!".to_string();
                let name = mode.title.clone();
                self.upsert_mode(mode.clone());
                self.selected = Some(mode.id.clone());
                self.modal = None;
                self.toast(format!("Режим «{name}» опубликован 🎉"), ToastKind::Ok);
            }
        }
        ctx.request_repaint();
    }

    fn upsert_mode(&mut self, mode: Mode) {
        if let Some(existing) = self.modes.iter_mut().find(|m| m.id == mode.id) {
            *existing = mode;
        } else {
            self.modes.push(mode);
        }
        self.modes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        let _ = self.store.cache_modes(&self.modes);
    }

    fn toast(&mut self, msg: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            msg: msg.into(),
            kind,
            born: self.time,
        });
    }

    fn selected_mode(&self) -> Option<&Mode> {
        let id = self.selected.as_deref()?;
        self.modes.iter().find(|m| m.id == id)
    }

    fn state_for(&self, mode: &Mode) -> Option<&ModeState> {
        self.store.state.get(&mode.id)
    }

    fn card_status(&self, mode: &Mode) -> (&'static str, &'static str) {
        if self.running.as_deref() == Some(mode.id.as_str()) {
            return ("ok", "В игре");
        }
        match self.state_for(mode) {
            Some(st) if st.installed => {
                if st.version.as_deref() != Some(mode.version.as_str()) {
                    ("update", "Обновление")
                } else {
                    ("ok", "Установлен")
                }
            }
            _ => ("new", "Новый"),
        }
    }

    fn action_for(&self, mode: &Mode) -> Action {
        if self.running.as_deref() == Some(mode.id.as_str()) {
            return Action::Running;
        }
        match self.state_for(mode) {
            Some(st) if st.installed => {
                if st.version.as_deref() != Some(mode.version.as_str()) {
                    Action::Update
                } else {
                    Action::Play
                }
            }
            _ => Action::Install,
        }
    }

    fn start_game(&mut self, mode: Mode, install_only: bool, ctx: &egui::Context) {
        if self.busy.is_some() {
            self.toast("Дождитесь завершения текущей операции", ToastKind::Warn);
            return;
        }
        let Some(account) = self.store.current_account() else {
            self.modal = Some(ModalKind::Account);
            self.toast("Сначала войдите в аккаунт Ely.by", ToastKind::Warn);
            return;
        };
        let settings = self.store.settings.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        self.busy = Some(Busy {
            mode_id: mode.id.clone(),
        });
        self.progress = 0.0;
        self.status = if install_only {
            "Загрузка файлов…".to_string()
        } else {
            "Подготовка к запуску…".to_string()
        };
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            game::start(
                mode,
                account,
                settings,
                install_only,
                tx,
                cancel,
                ctx,
            );
        });
    }

    fn cancel_current(&mut self, ctx: &egui::Context) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
            self.status = "Отмена…".to_string();
            ctx.request_repaint();
        }
    }

    fn save_window_size(&mut self, ctx: &egui::Context) {
        if self.time - self.last_size_save < 2.0 {
            return;
        }
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if maximized {
            return;
        }
        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
            let size = rect.size();
            if (size.x - self.last_inner_size.x).abs() > 2.0
                || (size.y - self.last_inner_size.y).abs() > 2.0
            {
                self.last_inner_size = size;
                self.last_size_save = self.time;
                if size.x >= 980.0 && size.y >= 640.0 {
                    self.store.settings.launcher_width = size.x;
                    self.store.settings.launcher_height = size.y;
                    let _ = self.store.save_settings();
                }
            }
        }
    }

    fn handle_resize(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        let m = 6.0;
        let left = pos.x <= screen.left() + m;
        let right = pos.x >= screen.right() - m;
        let top = pos.y <= screen.top() + m;
        let bottom = pos.y >= screen.bottom() - m;
        let dir = match (left, right, top, bottom) {
            (true, _, true, _) => Some(egui::ResizeDirection::NorthWest),
            (_, true, true, _) => Some(egui::ResizeDirection::NorthEast),
            (true, _, _, true) => Some(egui::ResizeDirection::SouthWest),
            (_, true, _, true) => Some(egui::ResizeDirection::SouthEast),
            (true, _, _, _) => Some(egui::ResizeDirection::West),
            (_, true, _, _) => Some(egui::ResizeDirection::East),
            (_, _, true, _) => Some(egui::ResizeDirection::North),
            (_, _, _, true) => Some(egui::ResizeDirection::South),
            _ => None,
        };
        if let Some(dir) = dir {
            ctx.set_cursor_icon(match dir {
                egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                    egui::CursorIcon::ResizeNwSe
                }
                egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
                    egui::CursorIcon::ResizeNeSw
                }
                egui::ResizeDirection::North | egui::ResizeDirection::South => {
                    egui::CursorIcon::ResizeVertical
                }
                _ => egui::CursorIcon::ResizeHorizontal,
            });
            if ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        }
    }

    fn draw_background(&self, ui: &mut Ui) {
        let screen = ui.max_rect();
        let painter = ui.painter().clone();
        painter.rect_filled(screen, 0, theme::BG);
        theme::vertical_gradient(
            &painter,
            screen,
            Color32::from_rgb(10, 10, 22),
            Color32::from_rgb(5, 5, 11),
        );
        let t = self.time as f32;
        let c1 = pos2(
            screen.left() + screen.width() * 0.10 + (t * 0.07).sin() * 45.0,
            screen.top() + screen.height() * 0.08 + (t * 0.05).cos() * 35.0,
        );
        let c2 = pos2(
            screen.right() - screen.width() * 0.05 + (t * 0.06).cos() * 55.0,
            screen.bottom() - screen.height() * 0.02 + (t * 0.04).sin() * 35.0,
        );
        let c3 = pos2(
            screen.left() + screen.width() * 0.48 + (t * 0.03).sin() * 70.0,
            screen.top() + screen.height() * 0.55,
        );
        theme::radial_glow(&painter, c1, 540.0, theme::alpha(theme::ACCENT, 46));
        theme::radial_glow(&painter, c2, 470.0, theme::alpha(theme::ACCENT_2, 24));
        theme::radial_glow(&painter, c3, 380.0, theme::alpha(theme::PINK, 18));

        // едва заметная сетка
        let step = 56.0;
        let grid = theme::alpha(Color32::WHITE, 5);
        let mut x = screen.left() - screen.left() % step;
        while x < screen.right() {
            painter.line_segment(
                [pos2(x, screen.top()), pos2(x, screen.bottom())],
                Stroke::new(1.0, grid),
            );
            x += step;
        }
        let mut y = screen.top() - screen.top() % step;
        while y < screen.bottom() {
            painter.line_segment(
                [pos2(screen.left(), y), pos2(screen.right(), y)],
                Stroke::new(1.0, grid),
            );
            y += step;
        }
    }

    fn draw_titlebar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("titlebar")
            .exact_size(TITLEBAR_HEIGHT)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(10, 10, 20, 210))
                    .inner_margin(Margin::symmetric(14, 0)),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter().clone();
                painter.line_segment(
                    [pos2(rect.left(), rect.bottom()), pos2(rect.right(), rect.bottom())],
                    Stroke::new(1.0, theme::alpha(Color32::WHITE, 12)),
                );

                let logo_rect = egui::Rect::from_center_size(
                    pos2(rect.left() + 18.0, rect.center().y),
                    vec2(30.0, 30.0),
                );
                theme::draw_logo(&painter, logo_rect, self.time as f32);
                let brand_x = logo_rect.right() + 10.0;
                painter.text(
                    pos2(brand_x, rect.center().y - 1.0),
                    Align2::LEFT_CENTER,
                    "TDEV",
                    theme::heading(11.0),
                    theme::MUTED,
                );
                let tdev_width = painter
                    .layout_no_wrap(
                        "TDEV".to_string(),
                        theme::heading(11.0),
                        Color32::WHITE,
                    )
                    .size()
                    .x;
                painter.text(
                    pos2(brand_x + tdev_width + 7.0, rect.center().y - 1.0),
                    Align2::LEFT_CENTER,
                    "LAUNCHER",
                    theme::heading(11.0),
                    theme::TEXT,
                );

                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                let mut x = rect.right() - 8.0;
                let close_rect = egui::Rect::from_min_size(
                    pos2(x - 38.0, rect.center().y - 16.0),
                    vec2(38.0, 32.0),
                );
                if widgets::icon_button(ui, close_rect, Icon::Close, true).clicked() {
                    self.save_window_size_now(&ctx);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                x -= 42.0;
                let max_rect = egui::Rect::from_min_size(
                    pos2(x - 38.0, rect.center().y - 16.0),
                    vec2(38.0, 32.0),
                );
                let max_icon = if maximized {
                    Icon::Restore
                } else {
                    Icon::Maximize
                };
                if widgets::icon_button(ui, max_rect, max_icon, false).clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                x -= 42.0;
                let min_rect = egui::Rect::from_min_size(
                    pos2(x - 38.0, rect.center().y - 16.0),
                    vec2(38.0, 32.0),
                );
                if widgets::icon_button(ui, min_rect, Icon::Minus, false).clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                x -= 46.0;
                let settings_rect = egui::Rect::from_min_size(
                    pos2(x - 38.0, rect.center().y - 16.0),
                    vec2(38.0, 32.0),
                );
                if widgets::icon_button(ui, settings_rect, Icon::Settings, false).clicked() {
                    self.open_modal(ModalKind::Settings);
                }
                x -= 12.0;

                let acc = self.store.current_account();
                let name = acc.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| "Гость".to_string());
                let chip_width = (widgets::text_width(ui, &name, theme::body(12.5)) + 72.0).clamp(120.0, 190.0);
                let chip_rect = egui::Rect::from_min_size(
                    pos2(x - chip_width, rect.center().y - 17.0),
                    vec2(chip_width, 34.0),
                );
                let chip_resp = ui.interact(chip_rect, Id::new("account-chip"), Sense::click());
                let chip_hover = ctx.animate_bool(chip_resp.id, chip_resp.hovered());
                let p = ui.painter().clone();
                p.rect_filled(
                    chip_rect,
                    17,
                    theme::mix(
                        theme::alpha(Color32::WHITE, 10),
                        theme::alpha(Color32::WHITE, 18),
                        chip_hover,
                    ),
                );
                p.rect_stroke(
                    chip_rect,
                    17,
                    Stroke::new(1.0, theme::mix(theme::BORDER, theme::ACCENT.gamma_multiply(0.6), chip_hover)),
                    StrokeKind::Inside,
                );
                let avatar_rect = egui::Rect::from_center_size(
                    pos2(chip_rect.left() + 17.0, chip_rect.center().y),
                    vec2(24.0, 24.0),
                );
                if let Some(acc) = &acc {
                    if let Some(url) = acc.skin_url() {
                        self.images.ensure(&url, &url, &ctx);
                        widgets::avatar(
                            ui,
                            avatar_rect,
                            self.images.texture(&url),
                            &acc.initial(),
                            theme::ACCENT,
                        );
                    } else {
                        widgets::avatar(ui, avatar_rect, None, &acc.initial(), theme::ACCENT);
                    }
                } else {
                    widgets::avatar(ui, avatar_rect, None, "Г", theme::MUTED_2);
                }
                p.text(
                    pos2(avatar_rect.right() + 8.0, chip_rect.center().y),
                    Align2::LEFT_CENTER,
                    shorten(&name, 18),
                    theme::body(12.5),
                    theme::TEXT,
                );
                widgets::draw_icon(
                    &p,
                    egui::Rect::from_center_size(
                        pos2(chip_rect.right() - 13.0, chip_rect.center().y),
                        vec2(12.0, 12.0),
                    ),
                    Icon::ChevronDown,
                    theme::MUTED,
                    1.3,
                );
                if chip_resp.clicked() {
                    self.open_modal(ModalKind::Account);
                }

                let drag_rect = egui::Rect::from_min_max(
                    pos2(rect.left() + 180.0, rect.top()),
                    pos2(chip_rect.left() - 8.0, rect.bottom()),
                );
                let drag = ui.interact(drag_rect, Id::new("titlebar-drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
            });
    }

    fn save_window_size_now(&mut self, ctx: &egui::Context) {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if maximized {
            return;
        }
        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
            let size = rect.size();
            if size.x >= 980.0 && size.y >= 640.0 {
                self.store.settings.launcher_width = size.x;
                self.store.settings.launcher_height = size.y;
                let _ = self.store.save_settings();
            }
        }
    }

    fn draw_sidebar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("РЕЖИМЫ")
                    .font(theme::heading(15.0))
                    .color(theme::TEXT),
            );
            ui.add_space(2.0);
            let count = self.modes.len();
            let text = count.to_string();
            let font = theme::body(11.0);
            let size = widgets::text_width(ui, &text, font.clone());
            let (rect, _) = ui.allocate_exact_size(vec2(size + 16.0, 20.0), Sense::hover());
            ui.painter().rect_filled(rect, 10, theme::alpha(Color32::WHITE, 12));
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                text,
                font,
                theme::MUTED,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let plus_rect = egui::Rect::from_center_size(
                    pos2(ui.max_rect().right() - 14.0, ui.max_rect().center().y),
                    vec2(28.0, 28.0),
                );
                if widgets::icon_button(ui, plus_rect, Icon::Plus, false).clicked() {
                    self.open_modal(ModalKind::Publish);
                }
                let refresh_rect = egui::Rect::from_center_size(
                    pos2(plus_rect.left() - 16.0, plus_rect.center().y),
                    vec2(28.0, 28.0),
                );
                if widgets::icon_button(ui, refresh_rect, Icon::Refresh, false).clicked()
                    && !self.refreshing
                {
                    self.refreshing = true;
                    self.spawn_modes_load(ui.ctx(), true);
                }
                if self.refreshing {
                    widgets::small_spinner(ui, self.time);
                }
            });
        });
        ui.add_space(12.0);

        let modes = self.modes.clone();
        let selected = self.selected.clone();
        if modes.is_empty() {
            widgets::empty_state(
                ui,
                "Режимов пока нет",
                "Нажми «+», чтобы выложить первый режим в репозиторий",
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("modes-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for mode in &modes {
                        self.draw_mode_card(ui, mode, selected.as_deref() == Some(mode.id.as_str()));
                        ui.add_space(10.0);
                    }
                    ui.add_space(2.0);
                });
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new("Папка игры")
                                .font(theme::body(12.0))
                                .color(theme::MUTED),
                        )
                        .sense(Sense::click()),
                    )
                    .clicked()
                {
                    let _ = open_path(std::path::Path::new(&self.store.settings.dir));
                }
                ui.label(RichText::new("•").font(theme::body(11.0)).color(theme::MUTED_2));
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new("tdev.gg")
                                .font(theme::body(12.0))
                                .color(theme::MUTED),
                        )
                        .sense(Sense::click()),
                    )
                    .clicked()
                {
                    let _ = open_url("https://tdev.gg");
                }
            });
        });
    }

    fn draw_mode_card(&mut self, ui: &mut Ui, mode: &Mode, selected: bool) {
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(width, 100.0), Sense::click());
        let hover = ui.ctx().animate_bool(response.id, response.hovered());
        let accent = theme::rgb(mode.accent_rgb());
        let painter = ui.painter().clone();

        painter.rect_filled(rect, 16, theme::CARD);
        if let Some(source) = mode.image_source() {
            let key = source.to_string();
            self.images.ensure(&key, source, ui.ctx());
            if let Some(tex) = self.images.texture(&key) {
                let uv = cover_uv(tex.size_vec2(), rect.size());
                egui::Image::new(egui::load::SizedTexture::new(tex.id(), tex.size_vec2()))
                    .uv(uv)
                    .corner_radius(16)
                    .paint_at(ui, rect);
            } else {
                self.draw_card_placeholder(ui, rect, accent);
            }
        } else {
            self.draw_card_placeholder(ui, rect, accent);
        }

        // затемнение слева, чтобы текст читался
        theme::horizontal_gradient(
            &painter,
            rect,
            Color32::from_rgba_unmultiplied(5, 5, 12, 225),
            Color32::from_rgba_unmultiplied(5, 5, 12, 10),
        );

        painter.text(
            pos2(rect.left() + 15.0, rect.center().y - 10.0),
            Align2::LEFT_CENTER,
            shorten(&mode.title, 26),
            theme::heading(12.5),
            Color32::WHITE,
        );
        painter.text(
            pos2(rect.left() + 15.0, rect.center().y + 10.0),
            Align2::LEFT_CENTER,
            format!("MC {} • {}", mode.mc_version, mode.loader_label()),
            theme::body(10.5),
            theme::MUTED,
        );

        let (status_kind, status_text) = self.card_status(mode);
        let (status_color, _status_bg) = theme::badge_color(status_kind);
        let font = theme::body(9.0);
        let text_w = widgets::text_width(ui, status_text, font.clone());
        let badge = egui::Rect::from_min_size(
            pos2(rect.right() - text_w - 26.0, rect.top() + 9.0),
            vec2(text_w + 16.0, 20.0),
        );
        painter.rect_filled(badge, 8, Color32::from_rgba_unmultiplied(10, 10, 20, 205));
        painter.rect_stroke(
            badge,
            8,
            Stroke::new(1.0, theme::alpha(status_color, 110)),
            StrokeKind::Inside,
        );
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            status_text,
            font,
            status_color,
        );

        if selected {
            painter.rect_stroke(
                rect,
                16,
                Stroke::new(1.8, accent),
                StrokeKind::Inside,
            );
            theme::shadow_rect(&painter, rect, 16, theme::alpha(accent, 30));
        } else if hover > 0.01 {
            painter.rect_stroke(
                rect,
                16,
                Stroke::new(1.0, theme::alpha(Color32::WHITE, (45.0 * hover) as u8)),
                StrokeKind::Inside,
            );
        }

        if response.clicked() {
            self.selected = Some(mode.id.clone());
            self.progress = 0.0;
            self.status = self.default_status(mode).to_string();
        }
    }

    fn draw_card_placeholder(&self, ui: &Ui, rect: egui::Rect, accent: Color32) {
        let painter = ui.painter().clone();
        painter.rect_filled(rect, 16, Color32::from_rgb(14, 14, 28));
        theme::radial_glow(
            &painter,
            rect.center() + vec2(rect.width() * 0.22, 0.0),
            rect.width() * 0.6,
            theme::alpha(accent, 46),
        );
        let logo_rect = egui::Rect::from_center_size(
            rect.center() + vec2(rect.width() * 0.27, 0.0),
            vec2(44.0, 44.0),
        );
        theme::draw_logo(&painter, logo_rect, self.time as f32);
    }

    fn default_status(&self, mode: &Mode) -> &'static str {
        if self.running.as_deref() == Some(mode.id.as_str()) {
            "Игра запущена"
        } else if self
            .state_for(mode)
            .map(|s| s.installed)
            .unwrap_or(false)
        {
            "Готов к запуску"
        } else {
            "Файлы не скачаны"
        }
    }

    fn draw_detail(&mut self, ui: &mut Ui) {
        let panel_rect = ui.available_rect_before_wrap();
        let painter = ui.painter().clone();
        painter.rect_filled(
            panel_rect,
            20,
            Color32::from_rgba_unmultiplied(255, 255, 255, 5),
        );
        painter.rect_stroke(
            panel_rect,
            20,
            Stroke::new(1.0, theme::alpha(Color32::WHITE, 18)),
            StrokeKind::Inside,
        );

        let inner = panel_rect.shrink(16.0);
        let Some(mode) = self.selected_mode().cloned() else {
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(110.0);
                    let (rect, _) = ui.allocate_exact_size(vec2(92.0, 92.0), Sense::hover());
                    widgets::icon_circle(ui, rect, Icon::Download, theme::ACCENT);
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new("Выбери режим слева")
                            .font(theme::heading(16.0))
                            .color(theme::TEXT),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Здесь появятся описание, версия и кнопка запуска")
                            .font(theme::body(12.5))
                            .color(theme::MUTED),
                    );
                });
            });
            return;
        };

        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            let footer_h = 78.0;
            let content_h = (ui.available_height() - footer_h - 8.0).max(140.0);
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), content_h),
                Layout::top_down(Align::Min),
                |ui| {
                    self.draw_hero(ui, &mode);
                    ui.add_space(12.0);
                    self.draw_meta(ui, &mode);
                    ui.add_space(10.0);
                    self.draw_description(ui, &mode);
                },
            );
            ui.add_space(8.0);
            self.draw_detail_footer(ui, &mode);
        });
    }

    fn draw_hero(&mut self, ui: &mut Ui, mode: &Mode) {
        let height = 255.0_f32.min((ui.available_height() * 0.48).max(180.0));
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let accent = theme::rgb(mode.accent_rgb());
        let painter = ui.painter().clone();

        painter.rect_filled(rect, 16, Color32::from_rgb(14, 14, 28));
        if let Some(source) = mode.image_source() {
            let key = source.to_string();
            self.images.ensure(&key, source, ui.ctx());
            if let Some(tex) = self.images.texture(&key) {
                let uv = cover_uv(tex.size_vec2(), rect.size());
                egui::Image::new(egui::load::SizedTexture::new(tex.id(), tex.size_vec2()))
                    .uv(uv)
                    .corner_radius(16)
                    .paint_at(ui, rect);
            } else {
                theme::radial_glow(
                    &painter,
                    rect.center() + vec2(rect.width() * 0.18, 0.0),
                    rect.width() * 0.55,
                    theme::alpha(accent, 52),
                );
                theme::draw_logo(
                    &painter,
                    egui::Rect::from_center_size(rect.center(), vec2(92.0, 92.0)),
                    self.time as f32,
                );
            }
        } else {
            theme::radial_glow(
                &painter,
                rect.center() + vec2(rect.width() * 0.18, 0.0),
                rect.width() * 0.55,
                theme::alpha(accent, 52),
            );
            theme::draw_logo(
                &painter,
                egui::Rect::from_center_size(rect.center(), vec2(92.0, 92.0)),
                self.time as f32,
            );
        }

        theme::vertical_gradient(
            &painter,
            rect,
            Color32::TRANSPARENT,
            Color32::from_rgba_unmultiplied(5, 5, 12, 230),
        );

        let title_font = theme::heading((rect.width() * 0.033).clamp(22.0, 32.0));
        painter.text(
            pos2(rect.left() + 22.0, rect.bottom() - 42.0),
            Align2::LEFT_BOTTOM,
            shorten(&mode.title, 42),
            title_font,
            Color32::WHITE,
        );

        let mut x = rect.left() + 22.0;
        let tag_y = rect.bottom() - 22.0;
        for tag in mode.tags.iter().take(4) {
            let font = theme::body(9.5);
            let w = widgets::text_width(ui, tag, font.clone());
            let tag_rect = egui::Rect::from_min_size(
                pos2(x, tag_y - 9.0),
                vec2(w + 18.0, 20.0),
            );
            painter.rect_filled(
                tag_rect,
                10,
                Color32::from_rgba_unmultiplied(10, 10, 20, 160),
            );
            painter.rect_stroke(
                tag_rect,
                10,
                Stroke::new(1.0, theme::alpha(Color32::WHITE, 50)),
                StrokeKind::Inside,
            );
            painter.text(
                tag_rect.center(),
                Align2::CENTER_CENTER,
                tag,
                font,
                theme::TEXT,
            );
            x += tag_rect.width() + 7.0;
        }

        if !mode.loader.is_empty() && mode.loader != "vanilla" {
            let font = theme::body(9.0);
            let label = format!("{} {}", mode.loader_label(), mode.loader_version.as_deref().unwrap_or(""));
            let w = widgets::text_width(ui, label.trim(), font.clone());
            let loader_rect = egui::Rect::from_min_size(
                pos2(rect.right() - w - 38.0, rect.top() + 14.0),
                vec2(w + 18.0, 21.0),
            );
            painter.rect_filled(loader_rect, 10, Color32::from_rgba_unmultiplied(10, 10, 20, 190));
            painter.text(
                loader_rect.center(),
                Align2::CENTER_CENTER,
                label.trim(),
                font,
                accent,
            );
        }
    }

    fn draw_meta(&mut self, ui: &mut Ui, mode: &Mode) {
        let values = [
            ("ВЕРСИЯ", format!("v{}", if mode.version.is_empty() { "—" } else { &mode.version })),
            ("MINECRAFT", if mode.mc_version.is_empty() { "—".to_string() } else { mode.mc_version.clone() }),
            ("ЗАГРУЗЧИК", mode.loader_title()),
            ("РАЗМЕР", mode.size.clone().unwrap_or_else(|| "—".to_string())),
            (
                "ОНЛАЙН",
                mode.players
                    .as_deref()
                    .filter(|p| *p != "—")
                    .map(|p| format!("{p} online"))
                    .unwrap_or_else(|| "—".to_string()),
            ),
        ];
        ui.columns(5, |cols| {
            for (i, col) in cols.iter_mut().enumerate() {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                    .stroke(Stroke::new(1.0, theme::alpha(Color32::WHITE, 14)))
                    .corner_radius(12)
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(col, |ui| {
                        widgets::key_value(ui, values[i].0, &values[i].1);
                    });
            }
        });
    }

    fn draw_description(&mut self, ui: &mut Ui, mode: &Mode) {
        egui::ScrollArea::vertical()
            .id_salt("detail-desc")
            .auto_shrink([false, false])
            .max_height(ui.available_height().max(80.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(if mode.description.trim().is_empty() {
                        "Описание не заполнено"
                    } else {
                        mode.description.trim()
                    })
                    .font(theme::body(13.0))
                    .color(Color32::from_rgb(198, 202, 228)),
                );
                if !mode.changelog.trim().is_empty() {
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        let (bar, _) = ui.allocate_exact_size(vec2(3.0, 18.0), Sense::hover());
                        ui.painter()
                            .rect_filled(bar, 2, theme::rgb(mode.accent_rgb()));
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("ЧТО НОВОГО")
                                .font(theme::body(9.5))
                                .color(theme::MUTED)
                                .strong(),
                        );
                    });
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(mode.changelog.trim())
                            .font(theme::body(12.0))
                            .color(Color32::from_rgb(174, 179, 212)),
                    );
                }
                ui.add_space(8.0);
            });
    }

    fn draw_detail_footer(&mut self, ui: &mut Ui, mode: &Mode) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 62.0), Sense::hover());
        let painter = ui.painter().clone();
        painter.line_segment(
            [pos2(rect.left(), rect.top()), pos2(rect.right(), rect.top())],
            Stroke::new(1.0, theme::alpha(Color32::WHITE, 14)),
        );

        let busy_here = self
            .busy
            .as_ref()
            .map(|b| b.mode_id == mode.id)
            .unwrap_or(false);
        let action = self.action_for(mode);

        let status_text = if busy_here {
            self.status.clone()
        } else if self.running.as_deref() == Some(mode.id.as_str()) {
            "Игра запущена".to_string()
        } else {
            self.default_status(mode).to_string()
        };
        let status_rect = egui::Rect::from_min_max(
            rect.left_top() + vec2(2.0, 14.0),
            pos2(rect.left() + 170.0, rect.bottom() - 8.0),
        );
        painter.text(
            status_rect.left_top(),
            Align2::LEFT_TOP,
            shorten(&status_text, 28),
            theme::body(12.0),
            theme::MUTED,
        );

        let button_width = 220.0;
        let button_rect = egui::Rect::from_min_size(
            pos2(rect.right() - button_width, rect.top() + 8.0),
            vec2(button_width, 44.0),
        );
        let progress_left = status_rect.right() + 16.0;
        let progress_right = button_rect.left() - 18.0;
        let show_progress = busy_here;
        if show_progress && progress_right > progress_left {
            let track = egui::Rect::from_min_size(
                pos2(progress_left, rect.center().y - 3.0),
                vec2(progress_right - progress_left, 6.0),
            );
            painter.rect_filled(track, 3, theme::alpha(Color32::WHITE, 12));
            let clip = painter.with_clip_rect(track);
            if self.progress > 0.001 {
                let fill = egui::Rect::from_min_size(
                    track.min,
                    vec2(track.width() * self.progress.clamp(0.0, 1.0), track.height()),
                );
                theme::horizontal_gradient(&clip, fill, theme::ACCENT, theme::ACCENT_2);
            } else {
                // неопределённый прогресс: бегущий блик, пока идёт установка
                let phase = (self.time as f32 * 0.55) % 1.0;
                let seg = track.width() * 0.28;
                let x = track.left() - seg + (track.width() + seg) * phase;
                let fill = egui::Rect::from_min_max(
                    pos2(x.max(track.left()), track.top()),
                    pos2((x + seg).min(track.right()), track.bottom()),
                );
                theme::horizontal_gradient(&clip, fill, theme::ACCENT, theme::ACCENT_2);
            }
        }

        let (label, color, enabled) = if busy_here {
            ("ОТМЕНИТЬ", theme::DANGER, true)
        } else {
            match action {
                Action::Install => ("УСТАНОВИТЬ", theme::ACCENT, true),
                Action::Update => ("ОБНОВИТЬ", theme::ORANGE, true),
                Action::Play => ("ИГРАТЬ", theme::OK, true),
                Action::Running => ("ИГРАЕТ…", theme::OK, false),
            }
        };

        // кнопка рисуется вручную, чтобы попасть в точный rect
        let response = ui.interact(button_rect, Id::new(("action", &mode.id)), Sense::click());
        let hover = ui
            .ctx()
            .animate_bool(response.id, response.hovered() && enabled);
        let base = if enabled {
            theme::mix(color, Color32::WHITE, 0.10 * hover)
        } else {
            theme::MUTED_2
        };
        if enabled {
            theme::shadow_rect(&painter, button_rect, 14, color);
        }
        painter.rect_filled(button_rect, 14, base);
        painter.text(
            button_rect.center(),
            Align2::CENTER_CENTER,
            label,
            theme::heading(12.5),
            if enabled { Color32::WHITE } else { theme::MUTED },
        );

        if response.clicked() && enabled {
            if busy_here {
                self.cancel_current(ui.ctx());
            } else if action != Action::Running {
                let install_only = action != Action::Play;
                self.start_game(mode.clone(), install_only, ui.ctx());
            }
        }
    }

    fn open_modal(&mut self, kind: ModalKind) {
        self.modal = Some(kind);
        match kind {
            ModalKind::Settings => {
                self.settings_form = SettingsForm::from_settings(&self.store.settings);
            }
            ModalKind::Account => {
                self.login_error = None;
                self.offline_error = None;
                self.login_busy = false;
            }
            ModalKind::Publish => {
                self.publish_form.repo = self.store.settings.github_repo.clone();
                self.publish_form.token = self.store.settings.github_token.clone();
                self.publish_error = None;
                self.publish_progress = 0.0;
                self.publish_step.clear();
            }
        }
    }

    fn draw_modals(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.modal.is_some() {
                self.modal = None;
            }
        }
        match self.modal {
            Some(ModalKind::Settings) => self.draw_settings_modal(ctx),
            Some(ModalKind::Account) => self.draw_account_modal(ctx),
            Some(ModalKind::Publish) => self.draw_publish_modal(ctx),
            None => {}
        }
    }

    fn modal_header(ui: &mut Ui, title: &str, close: &mut bool) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::heading(15.0))
                    .color(theme::TEXT),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let rect = egui::Rect::from_center_size(
                    pos2(ui.max_rect().right() - 15.0, ui.max_rect().center().y),
                    vec2(30.0, 30.0),
                );
                if widgets::icon_button(ui, rect, Icon::Close, false).clicked() {
                    *close = true;
                }
            });
        });
        ui.add_space(14.0);
    }

    fn draw_settings_modal(&mut self, ctx: &egui::Context) {
        let mut form = self.settings_form.clone();
        let mut close = false;
        let mut save = false;
        let mut choose_dir = false;
        let mut open_dir = false;

        egui::Modal::new(Id::new("settings-modal"))
            .frame(widgets::modal_frame())
            .show(ctx, |ui| {
                ui.set_width(560.0);
                Self::modal_header(ui, "НАСТРОЙКИ", &mut close);
                egui::ScrollArea::vertical()
                    .max_height(520.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Оперативная память")
                                .font(theme::body(10.5))
                                .color(theme::MUTED)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Slider::new(&mut form.ram_gb, 2..=32)
                                    .suffix(" ГБ")
                                    .show_value(true),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Сколько ОЗУ отдать игре. 4–8 ГБ — нормально для сборок")
                                .font(theme::body(11.0))
                                .color(theme::MUTED_2),
                        );
                        ui.add_space(16.0);

                        ui.label(
                            RichText::new("Папка игры")
                                .font(theme::body(10.5))
                                .color(theme::MUTED)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 190.0).max(180.0);
                            Frame::new()
                                .fill(theme::alpha(Color32::WHITE, 10))
                                .stroke(Stroke::new(1.0, theme::BORDER))
                                .corner_radius(10)
                                .inner_margin(Margin::symmetric(12, 9))
                                .show(ui, |ui| {
                                    ui.add_sized(
                                        [width, 20.0],
                                        egui::Label::new(
                                            RichText::new(shorten(&form.dir, 52))
                                                .font(theme::body(12.5))
                                                .color(theme::MUTED),
                                        ),
                                    );
                                });
                            if widgets::ghost_button(ui, "Выбрать").clicked() {
                                choose_dir = true;
                            }
                            if widgets::ghost_button(ui, "Открыть").clicked() {
                                open_dir = true;
                            }
                        });
                        ui.add_space(16.0);

                        widgets::text_field(
                            ui,
                            "Путь к Java",
                            &mut form.java_path,
                            "пусто = системная java",
                            false,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Например: C:\\Program Files\\Java\\jdk-17\\bin\\javaw.exe")
                                .font(theme::body(11.0))
                                .color(theme::MUTED_2),
                        );
                        ui.add_space(16.0);

                        ui.label(
                            RichText::new("Окно игры")
                                .font(theme::body(10.5))
                                .color(theme::MUTED)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let mut w = form.width.parse::<u32>().unwrap_or(925);
                            let mut h = form.height.parse::<u32>().unwrap_or(530);
                            ui.add(
                                egui::DragValue::new(&mut w)
                                    .range(400..=3840)
                                    .speed(1.0)
                                    .prefix("Ш: "),
                            );
                            ui.add(
                                egui::DragValue::new(&mut h)
                                    .range(300..=2160)
                                    .speed(1.0)
                                    .prefix("В: "),
                            );
                            form.width = w.to_string();
                            form.height = h.to_string();
                            ui.checkbox(&mut form.fullscreen, "Полный экран");
                        });
                        ui.add_space(8.0);
                    });
                ui.add_space(8.0);
                if widgets::accent_button(ui, "СОХРАНИТЬ", ui.available_width(), 44.0, theme::ACCENT, true)
                    .clicked()
                {
                    save = true;
                }
            });

        if choose_dir {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                form.dir = path_string(&dir);
            }
        }
        if open_dir {
            let _ = open_path(std::path::Path::new(&form.dir));
        }
        if save {
            let width = form.width.parse::<u32>().unwrap_or(925).clamp(400, 3840);
            let height = form.height.parse::<u32>().unwrap_or(530).clamp(300, 2160);
            self.store.settings.ram_gb = form.ram_gb.clamp(2, 32);
            self.store.settings.dir = if form.dir.trim().is_empty() {
                self.store.settings.dir.clone()
            } else {
                form.dir.trim().to_string()
            };
            self.store.settings.java_path = form.java_path.trim().to_string();
            self.store.settings.width = width;
            self.store.settings.height = height;
            self.store.settings.fullscreen = form.fullscreen;
            let _ = self.store.save_settings();
            self.settings_form = SettingsForm::from_settings(&self.store.settings);
            self.modal = None;
            self.toast("Настройки сохранены", ToastKind::Ok);
        } else if close {
            self.modal = None;
        }
        self.settings_form = form;
    }

    fn draw_account_modal(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut login = false;
        let mut offline = false;
        let mut select_account: Option<String> = None;
        let mut delete_account: Option<String> = None;
        let current = self.store.current_account();
        let accounts = self.store.accounts.clone();

        egui::Modal::new(Id::new("account-modal"))
            .frame(widgets::modal_frame())
            .show(ctx, |ui| {
                ui.set_width(560.0);
                Self::modal_header(ui, "АККАУНТ", &mut close);

                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 66.0), Sense::hover());
                let painter = ui.painter().clone();
                painter.rect_filled(rect, 14, theme::alpha(Color32::WHITE, 8));
                painter.rect_stroke(
                    rect,
                    14,
                    Stroke::new(1.0, theme::alpha(Color32::WHITE, 16)),
                    StrokeKind::Inside,
                );
                let avatar_rect = egui::Rect::from_center_size(
                    pos2(rect.left() + 32.0, rect.center().y),
                    vec2(38.0, 38.0),
                );
                if let Some(acc) = &current {
                    if let Some(url) = acc.skin_url() {
                        self.images.ensure(&url, &url, &ctx);
                        widgets::avatar(
                            ui,
                            avatar_rect,
                            self.images.texture(&url),
                            &acc.initial(),
                            theme::ACCENT,
                        );
                    } else {
                        widgets::avatar(ui, avatar_rect, None, &acc.initial(), theme::ACCENT);
                    }
                    painter.text(
                        pos2(avatar_rect.right() + 12.0, rect.center().y - 9.0),
                        Align2::LEFT_CENTER,
                        &acc.name,
                        theme::body(15.0),
                        theme::TEXT,
                    );
                    let sub = if acc.is_ely() {
                        if acc.session_error {
                            "Сессия истекла — войдите заново"
                        } else {
                            "Аккаунт Ely.by"
                        }
                    } else {
                        "Офлайн-аккаунт"
                    };
                    painter.text(
                        pos2(avatar_rect.right() + 12.0, rect.center().y + 10.0),
                        Align2::LEFT_CENTER,
                        sub,
                        theme::body(11.5),
                        if acc.session_error {
                            theme::WARN
                        } else {
                            theme::MUTED
                        },
                    );
                } else {
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "Аккаунтов нет — войди через Ely.by или создай офлайн-профиль",
                        theme::body(12.0),
                        theme::MUTED,
                    );
                }
                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    if Self::tab_button(ui, "Ely.by", self.account_tab == AccountTab::Ely, w) {
                        self.account_tab = AccountTab::Ely;
                    }
                    if Self::tab_button(ui, "Офлайн", self.account_tab == AccountTab::Offline, w) {
                        self.account_tab = AccountTab::Offline;
                    }
                });
                ui.add_space(14.0);

                if self.account_tab == AccountTab::Ely {
                    widgets::text_field(ui, "Логин или e-mail Ely.by", &mut self.login_user, "", false);
                    ui.add_space(8.0);
                    widgets::text_field(ui, "Пароль", &mut self.login_pass, "", true);
                    if let Some(err) = &self.login_error {
                        ui.add_space(6.0);
                        ui.label(RichText::new(err).font(theme::body(11.5)).color(theme::DANGER));
                    }
                    ui.add_space(12.0);
                    let text = if self.login_busy { "ВХОД…" } else { "ВОЙТИ" };
                    if widgets::accent_button(
                        ui,
                        text,
                        ui.available_width(),
                        42.0,
                        theme::ACCENT,
                        !self.login_busy,
                    )
                    .clicked()
                    {
                        login = true;
                    }
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new("Войти через сайт Ely.by")
                                        .font(theme::body(11.0))
                                        .color(theme::ACCENT_2),
                                )
                                .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            let _ = open_url("https://account.ely.by/auth/login");
                        }
                        ui.label(
                            RichText::new("Нет аккаунта? Регистрация на ely.by — бесплатно")
                                .font(theme::body(10.5))
                                .color(theme::MUTED_2),
                        );
                    });
                } else {
                    widgets::text_field(ui, "Ник для игры", &mut self.offline_name, "3–16 символов", false);
                    if let Some(err) = &self.offline_error {
                        ui.add_space(6.0);
                        ui.label(RichText::new(err).font(theme::body(11.5)).color(theme::DANGER));
                    }
                    ui.add_space(12.0);
                    if widgets::accent_button(
                        ui,
                        "ИГРАТЬ БЕЗ ВХОДА",
                        ui.available_width(),
                        42.0,
                        theme::ACCENT_2,
                        true,
                    )
                    .clicked()
                    {
                        offline = true;
                    }
                    ui.add_space(6.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Скины и серверы с авторизацией будут недоступны")
                                .font(theme::body(10.5))
                                .color(theme::MUTED_2),
                        );
                    });
                }

                ui.add_space(18.0);
                widgets::divider(ui);
                ui.add_space(12.0);
                widgets::section_label(ui, "СОХРАНЁННЫЕ АККАУНТЫ");
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(170.0)
                    .id_salt("accounts-list")
                    .show(ui, |ui| {
                        if accounts.is_empty() {
                            ui.label(
                                RichText::new("Пока пусто")
                                    .font(theme::body(12.0))
                                    .color(theme::MUTED_2),
                            );
                        }
                        for acc in &accounts {
                            let is_current = current.as_ref().map(|c| c.id == acc.id).unwrap_or(false);
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 46.0),
                                Sense::hover(),
                            );
                            let p = ui.painter().clone();
                            p.rect_filled(
                                rect,
                                12,
                                if is_current {
                                    theme::alpha(theme::ACCENT, 26)
                                } else {
                                    theme::alpha(Color32::WHITE, 7)
                                },
                            );
                            p.rect_stroke(
                                rect,
                                12,
                                Stroke::new(
                                    1.0,
                                    if is_current {
                                        theme::alpha(theme::ACCENT, 140)
                                    } else {
                                        theme::alpha(Color32::WHITE, 12)
                                    },
                                ),
                                StrokeKind::Inside,
                            );
                            let avatar_rect = egui::Rect::from_center_size(
                                pos2(rect.left() + 23.0, rect.center().y),
                                vec2(30.0, 30.0),
                            );
                            if let Some(url) = acc.skin_url() {
                                self.images.ensure(&url, &url, &ctx);
                                widgets::avatar(
                                    ui,
                                    avatar_rect,
                                    self.images.texture(&url),
                                    &acc.initial(),
                                    theme::ACCENT,
                                );
                            } else {
                                widgets::avatar(ui, avatar_rect, None, &acc.initial(), theme::MUTED_2);
                            }
                            p.text(
                                pos2(avatar_rect.right() + 10.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                &acc.name,
                                theme::body(12.5),
                                theme::TEXT,
                            );
                            let type_text = if acc.is_ely() { "ely.by" } else { "офлайн" };
                            let type_x = rect.left() + 180.0;
                            p.text(
                                pos2(type_x, rect.center().y),
                                Align2::LEFT_CENTER,
                                type_text,
                                theme::body(9.5),
                                if acc.is_ely() { theme::ACCENT_2 } else { theme::MUTED },
                            );
                            let pick_rect = egui::Rect::from_min_size(
                                pos2(rect.right() - 126.0, rect.center().y - 14.0),
                                vec2(76.0, 28.0),
                            );
                            let pick_resp = ui.interact(
                                pick_rect,
                                Id::new(("pick", &acc.id)),
                                Sense::click(),
                            );
                            p.rect_filled(
                                pick_rect,
                                8,
                                if is_current {
                                    theme::alpha(theme::OK, 28)
                                } else {
                                    theme::alpha(Color32::WHITE, 10)
                                },
                            );
                            p.text(
                                pick_rect.center(),
                                Align2::CENTER_CENTER,
                                if is_current { "Выбран" } else { "Выбрать" },
                                theme::body(10.5),
                                if is_current { theme::OK } else { theme::TEXT },
                            );
                            if pick_resp.clicked() && !is_current {
                                select_account = Some(acc.id.clone());
                            }
                            let del_rect = egui::Rect::from_min_size(
                                pos2(rect.right() - 42.0, rect.center().y - 14.0),
                                vec2(28.0, 28.0),
                            );
                            if widgets::icon_button(ui, del_rect, Icon::Close, true).clicked() {
                                delete_account = Some(acc.id.clone());
                            }
                            ui.add_space(6.0);
                        }
                    });
            });

        if login {
            if self.login_user.trim().is_empty() || self.login_pass.is_empty() {
                self.login_error = Some("Введите логин и пароль".to_string());
            } else {
                self.login_busy = true;
                self.login_error = None;
                let username = self.login_user.trim().to_string();
                let password = self.login_pass.clone();
                let client_token = self.store.client_token.clone();
                let tx = self.tx.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let result = (|| -> Result<Account, String> {
                        let data = auth::ely_login(&username, &password, &client_token)?;
                        let profile = auth::pick_profile(&data, &username);
                        Ok(Account {
                            id: uuid::Uuid::new_v4().to_string(),
                            kind: "ely".to_string(),
                            name: profile.name,
                            uuid: profile.id,
                            access_token: data.access_token,
                            client_token: data.client_token.unwrap_or(client_token),
                            added_at: now_ms(),
                            session_error: false,
                        })
                    })();
                    let _ = tx.send(AppEvent::ElyLogin(result));
                    ctx.request_repaint();
                });
            }
        }
        if offline {
            let name = self.offline_name.trim().to_string();
            if !auth::valid_offline_name(&name) {
                self.offline_error = Some("Ник: 3–16 символов, без пробелов".to_string());
            } else {
                let account = Account {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: "offline".to_string(),
                    name: name.clone(),
                    uuid: auth::offline_uuid(&name),
                    access_token: "0".to_string(),
                    client_token: self.store.client_token.clone(),
                    added_at: now_ms(),
                    session_error: false,
                };
                let _ = self.store.upsert_account(account.clone());
                let _ = self.store.set_current_account(Some(account.id.clone()));
                self.offline_error = None;
                self.toast(format!("Офлайн-профиль: {name}"), ToastKind::Ok);
            }
        }
        if let Some(id) = select_account {
            let name = self
                .store
                .accounts
                .iter()
                .find(|a| a.id == id)
                .map(|a| a.name.clone())
                .unwrap_or_default();
            let _ = self.store.set_current_account(Some(id));
            self.session_checked = false;
            self.spawn_session_check(ctx);
            self.toast(format!("Аккаунт: {name}"), ToastKind::Ok);
        }
        if let Some(id) = delete_account {
            let _ = self.store.remove_account(&id);
            self.session_checked = false;
            self.spawn_session_check(ctx);
            self.toast("Аккаунт удалён", ToastKind::Info);
        }
        if close {
            self.modal = None;
        }
    }

    fn tab_button(ui: &mut Ui, text: &str, active: bool, width: f32) -> bool {
        let (rect, response) = ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
        let hover = ui.ctx().animate_bool(response.id, response.hovered());
        let painter = ui.painter().clone();
        painter.rect_filled(
            rect,
            10,
            if active {
                theme::ACCENT
            } else {
                theme::mix(
                    theme::alpha(Color32::WHITE, 7),
                    theme::alpha(Color32::WHITE, 13),
                    hover,
                )
            },
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            theme::body(12.5),
            if active { Color32::WHITE } else { theme::MUTED },
        );
        response.clicked()
    }

    fn draw_publish_modal(&mut self, ctx: &egui::Context) {
        let mut form = self.publish_form.clone();
        let mut close = false;
        let mut submit = false;
        let mut pick_art = false;
        let mut pick_zip = false;

        egui::Modal::new(Id::new("publish-modal"))
            .frame(widgets::modal_frame())
            .show(ctx, |ui| {
                ui.set_width(620.0);
                Self::modal_header(ui, "ВЫКЛАДКА РЕЖИМА", &mut close);
                egui::ScrollArea::vertical()
                    .max_height(520.0)
                    .id_salt("publish-scroll")
                    .show(ui, |ui| {
                        widgets::text_field(ui, "GitHub репозиторий", &mut form.repo, "owner/repo", false);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Сборка зальётся в modes/<id>/ пофайлово + манифест, арт в arts/, список — modes.json")
                                .font(theme::body(10.5))
                                .color(theme::MUTED_2),
                        );
                        ui.add_space(12.0);

                        widgets::text_field(ui, "GitHub токен", &mut form.token, "ghp_…", true);
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new("Создать токен")
                                            .font(theme::body(10.5))
                                            .color(theme::ACCENT_2),
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            {
                                let _ = open_url("https://github.com/settings/tokens/new?scopes=repo&description=TDEV%20Launcher");
                            }
                            ui.label(
                                RichText::new("→ галочка repo. Токен хранится только у тебя на ПК")
                                    .font(theme::body(10.5))
                                    .color(theme::MUTED_2),
                            );
                        });
                        ui.add_space(12.0);

                        widgets::text_field(ui, "Название режима", &mut form.title, "например, FROST LEGENDS", false);
                        ui.add_space(12.0);

                        ui.columns(2, |cols| {
                            widgets::text_field(&mut cols[0], "Версия режима", &mut form.version, "1.0.0", false);
                            widgets::text_field(&mut cols[1], "Версия Minecraft", &mut form.mc_version, "1.20.1", false);
                        });
                        ui.add_space(12.0);

                        ui.columns(2, |cols| {
                            cols[0].label(
                                RichText::new("Загрузчик")
                                    .font(theme::body(10.5))
                                    .color(theme::MUTED)
                                    .strong(),
                            );
                            cols[0].add_space(6.0);
                            egui::ComboBox::from_id_salt("pub-loader")
                                .width(cols[0].available_width())
                                .selected_text(match form.loader.as_str() {
                                    "forge" => "Forge",
                                    "fabric" => "Fabric",
                                    _ => "Vanilla",
                                })
                                .show_ui(&mut cols[0], |ui| {
                                    ui.selectable_value(&mut form.loader, "vanilla".to_string(), "Vanilla");
                                    ui.selectable_value(&mut form.loader, "forge".to_string(), "Forge");
                                    ui.selectable_value(&mut form.loader, "fabric".to_string(), "Fabric");
                                });
                            if form.loader != "vanilla" {
                                let mut lv = form.loader_version.clone().unwrap_or_default();
                                widgets::text_field(
                                    &mut cols[1],
                                    "Версия загрузчика",
                                    &mut lv,
                                    "пусто = рекомендованная",
                                    false,
                                );
                                form.loader_version = if lv.trim().is_empty() {
                                    None
                                } else {
                                    Some(lv.trim().to_string())
                                };
                            }
                        });
                        ui.add_space(12.0);

                        let mut tags = form.tags.join(", ");
                        widgets::text_field(ui, "Теги (через запятую)", &mut tags, "Хоррор, Кооператив", false);
                        form.tags = tags
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .collect();
                        ui.add_space(12.0);

                        widgets::text_area(ui, "Описание", &mut form.description, "О чём режим, что делать игрокам…", 3);
                        ui.add_space(12.0);
                        widgets::text_area(ui, "Что нового", &mut form.changelog, "• Добавлено…\n• Исправлено…", 2);
                        ui.add_space(12.0);

                        ui.columns(2, |cols| {
                            cols[0].label(
                                RichText::new("Цвет акцента")
                                    .font(theme::body(10.5))
                                    .color(theme::MUTED)
                                    .strong(),
                            );
                            cols[0].add_space(6.0);
                            let (r, g, b) = parse_hex_color(&form.accent).unwrap_or((139, 92, 246));
                            let mut rgb = [r, g, b];
                            if cols[0].color_edit_button_srgb(&mut rgb).changed() {
                                form.accent = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                            }
                            cols[1].label(
                                RichText::new("Арт карточки")
                                    .font(theme::body(10.5))
                                    .color(theme::MUTED)
                                    .strong(),
                            );
                            cols[1].add_space(6.0);
                            cols[1].horizontal(|ui| {
                                if widgets::ghost_button(ui, "Выбрать").clicked() {
                                    pick_art = true;
                                }
                                if let Some(path) = &form.image_path {
                                    let key = path_string(path);
                                    self.images.load_local(&key, path, ctx);
                                    if let Some(tex) = self.images.texture(&key) {
                                        let rect = egui::Rect::from_min_size(
                                            ui.cursor().min,
                                            vec2(62.0, 38.0),
                                        );
                                        egui::Image::new(egui::load::SizedTexture::new(
                                            tex.id(),
                                            tex.size_vec2(),
                                        ))
                                        .corner_radius(8)
                                        .paint_at(ui, rect);
                                        ui.allocate_space(vec2(62.0, 38.0));
                                    }
                                }
                            });
                            if !self.pub_art_name.is_empty() {
                                cols[1].label(
                                    RichText::new(&self.pub_art_name)
                                        .font(theme::body(10.0))
                                        .color(theme::MUTED_2),
                                );
                            }
                        });
                        ui.add_space(12.0);

                        ui.label(
                            RichText::new("Сборка (zip) — необязательно")
                                .font(theme::body(10.5))
                                .color(theme::MUTED)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if widgets::ghost_button(ui, "Выбрать zip").clicked() {
                                pick_zip = true;
                            }
                            if !self.pub_zip_name.is_empty() {
                                ui.label(
                                    RichText::new(&self.pub_zip_name)
                                        .font(theme::body(11.0))
                                        .color(theme::MUTED),
                                );
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("mods/, config/, resourcepacks/ и т.п. Зальётся пофайлово — обновления у игроков будут инкрементальными. Без zip — чистая ваниль")
                                .font(theme::body(10.5))
                                .color(theme::MUTED_2),
                        );
                        ui.add_space(14.0);

                        if self.publish_busy || !self.publish_step.is_empty() {
                            Frame::new()
                                .fill(theme::alpha(Color32::WHITE, 6))
                                .corner_radius(12)
                                .inner_margin(Margin::symmetric(12, 10))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if self.publish_busy {
                                            widgets::small_spinner(ui, self.time);
                                        } else {
                                            widgets::icon_circle(
                                                ui,
                                                egui::Rect::from_min_size(ui.cursor().min, vec2(18.0, 18.0)),
                                                Icon::Check,
                                                theme::OK,
                                            );
                                            ui.allocate_space(vec2(18.0, 18.0));
                                        }
                                        ui.label(
                                            RichText::new(&self.publish_step)
                                                .font(theme::body(11.5))
                                                .color(theme::ACCENT_2),
                                        );
                                    });
                                    ui.add_space(6.0);
                                    widgets::progress_bar(ui, self.publish_progress);
                                });
                            ui.add_space(10.0);
                        }
                        if let Some(err) = &self.publish_error {
                            ui.label(RichText::new(err).font(theme::body(11.5)).color(theme::DANGER));
                            ui.add_space(8.0);
                        }
                    });
                ui.add_space(8.0);
                let text = if self.publish_busy { "ПУБЛИКАЦИЯ…" } else { "ОПУБЛИКОВАТЬ" };
                if widgets::accent_button(
                    ui,
                    text,
                    ui.available_width(),
                    44.0,
                    theme::ACCENT,
                    !self.publish_busy,
                )
                .clicked()
                {
                    submit = true;
                }
            });

        if pick_art {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "svg"])
                .pick_file()
            {
                self.pub_art_name = crate::util::file_name(&path);
                form.image_path = Some(path);
            }
        }
        if pick_zip {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Архив ZIP", &["zip"])
                .pick_file()
            {
                self.pub_zip_name = format!(
                    "{} ({})",
                    crate::util::file_name(&path),
                    human_size(std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0))
                );
                form.zip_path = Some(path);
            }
        }
        if submit {
            if self.publish_busy {
                self.toast("Публикация уже идёт", ToastKind::Warn);
            } else {
                form.repo = form.repo.trim().to_string();
                form.token = form.token.trim().to_string();
                form.title = form.title.trim().to_string();
                form.version = form.version.trim().to_string();
                form.mc_version = form.mc_version.trim().to_string();
                form.description = form.description.trim().to_string();
                form.changelog = form.changelog.trim().to_string();
                self.publish_form = form.clone();
                self.store.settings.github_repo = form.repo.clone();
                self.store.settings.github_token = form.token.clone();
                let _ = self.store.save_settings();
                self.publish_busy = true;
                self.publish_error = None;
                self.publish_progress = 0.0;
                self.publish_step = "Старт…".to_string();
                let tx = self.tx.clone();
                let ctx2 = ctx.clone();
                let form_for_thread = form.clone();
                std::thread::spawn(move || {
                    if let Err(err) = publish::publish(&form_for_thread, &tx, &ctx2) {
                        let _ = tx.send(AppEvent::Publish(PublishEvent::Error(err)));
                        ctx2.request_repaint();
                    }
                });
            }
        }
        if close && !self.publish_busy {
            self.modal = None;
        }
        self.publish_form = form;
    }

    fn draw_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = self.toast.clone() else {
            return;
        };
        let age = (self.time - toast.born) as f32;
        if age > 3.6 {
            self.toast = None;
            return;
        }
        let fade = if age < 0.18 {
            age / 0.18
        } else if age > 3.2 {
            (3.6 - age) / 0.4
        } else {
            1.0
        }
        .clamp(0.0, 1.0);
        let offset = (1.0 - fade) * 18.0;
        let color = match toast.kind {
            ToastKind::Ok => theme::OK,
            ToastKind::Warn => theme::WARN,
            ToastKind::Error => theme::DANGER,
            ToastKind::Info => theme::ACCENT_2,
        };
        egui::Area::new(Id::new("toast-area"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -26.0 + offset))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(18, 18, 31, (238.0 * fade) as u8))
                    .stroke(Stroke::new(1.0, theme::alpha(Color32::WHITE, (40.0 * fade) as u8)))
                    .corner_radius(13)
                    .inner_margin(Margin::symmetric(18, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, color);
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(&toast.msg)
                                    .font(theme::body(12.5))
                                    .color(theme::alpha(theme::TEXT, (255.0 * fade) as u8)),
                            );
                        });
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.time = ctx.input(|i| i.time);
        self.poll_events(&ctx);
        self.handle_resize(&ctx);
        self.draw_background(ui);
        self.draw_titlebar(ui);

        egui::Panel::left("modes-panel")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE.inner_margin(Margin::symmetric(18, 14)))
            .show(ui, |ui| self.draw_sidebar(ui));

        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::symmetric(14, 14)))
            .show(ui, |ui| self.draw_detail(ui));

        self.draw_modals(&ctx);
        self.draw_toast(&ctx);
        self.save_window_size(&ctx);

        if self.busy.is_some()
            || self.publish_busy
            || self.refreshing
            || self.toast.is_some()
            || self.modal.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }
}
