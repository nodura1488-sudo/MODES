use crate::images::face_uv;
use crate::theme;
use eframe::egui;
use egui::{
    pos2, vec2, Align2, Color32, FontId, Frame, Margin, Response, RichText, Sense, Stroke, StrokeKind,
    TextEdit, TextureHandle, Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Settings,
    Refresh,
    Plus,
    Minus,
    Maximize,
    Restore,
    Close,
    ChevronDown,
    Check,
    Download,
}

pub fn draw_icon(painter: &egui::Painter, rect: egui::Rect, icon: Icon, color: Color32, width: f32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.32;
    let stroke = Stroke::new(width, color);
    match icon {
        Icon::Settings => {
            painter.circle_stroke(c, r * 0.42, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let p1 = c + vec2(a.cos(), a.sin()) * (r * 0.7);
                let p2 = c + vec2(a.cos(), a.sin()) * (r * 1.05);
                painter.line_segment([p1, p2], stroke);
            }
        }
        Icon::Refresh => {
            let radius = r * 0.9;
            let start = -std::f32::consts::FRAC_PI_2 + 0.5;
            let mut prev = c + vec2(start.cos(), start.sin()) * radius;
            let steps = 18;
            for i in 1..=steps {
                let a = start + i as f32 / steps as f32 * 4.7;
                let p = c + vec2(a.cos(), a.sin()) * radius;
                painter.line_segment([prev, p], stroke);
                prev = p;
            }
            painter.line_segment([prev, prev + vec2(-r * 0.42, -r * 0.12)], stroke);
            painter.line_segment([prev, prev + vec2(r * 0.12, -r * 0.45)], stroke);
        }
        Icon::Plus => {
            painter.line_segment([c - vec2(r, 0.0), c + vec2(r, 0.0)], stroke);
            painter.line_segment([c - vec2(0.0, r), c + vec2(0.0, r)], stroke);
        }
        Icon::Minus => {
            painter.line_segment([c - vec2(r, 0.0), c + vec2(r, 0.0)], stroke);
        }
        Icon::Maximize => {
            painter.rect_stroke(
                egui::Rect::from_center_size(c, vec2(r * 1.55, r * 1.55)),
                2,
                stroke,
                StrokeKind::Inside,
            );
        }
        Icon::Restore => {
            let a = egui::Rect::from_center_size(c + vec2(-r * 0.12, -r * 0.12), vec2(r * 1.3, r * 1.3));
            let b = egui::Rect::from_center_size(c + vec2(r * 0.18, r * 0.18), vec2(r * 1.3, r * 1.3));
            painter.rect_stroke(a, 2, stroke, StrokeKind::Inside);
            painter.rect_filled(b, 2, theme::PANEL);
            painter.rect_stroke(b, 2, stroke, StrokeKind::Inside);
        }
        Icon::Close => {
            painter.line_segment([c - vec2(r, r), c + vec2(r, r)], stroke);
            painter.line_segment([c + vec2(r, -r), c - vec2(r, -r)], stroke);
        }
        Icon::ChevronDown => {
            painter.line_segment([c + vec2(-r * 0.65, -r * 0.3), c + vec2(0.0, r * 0.35)], stroke);
            painter.line_segment([c + vec2(r * 0.65, -r * 0.3), c + vec2(0.0, r * 0.35)], stroke);
        }
        Icon::Check => {
            painter.line_segment([c + vec2(-r, 0.0), c + vec2(-r * 0.15, r * 0.8)], stroke);
            painter.line_segment([c + vec2(-r * 0.15, r * 0.8), c + vec2(r, -r * 0.8)], stroke);
        }
        Icon::Download => {
            painter.line_segment([c + vec2(0.0, -r), c + vec2(0.0, r * 0.4)], stroke);
            painter.line_segment([c + vec2(-r * 0.6, -r * 0.15), c + vec2(0.0, r * 0.45)], stroke);
            painter.line_segment([c + vec2(r * 0.6, -r * 0.15), c + vec2(0.0, r * 0.45)], stroke);
            painter.line_segment([c + vec2(-r * 0.8, r), c + vec2(r * 0.8, r)], stroke);
        }
    }
}

pub fn icon_button(ui: &mut Ui, rect: egui::Rect, icon: Icon, danger: bool) -> Response {
    let response = ui.interact(rect, ui.scope_id().with(("icon", icon as u8, rect.min.x as i32)), Sense::click());
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let painter = ui.painter();
    if hover > 0.01 {
        let bg = if danger {
            theme::alpha(theme::DANGER, (44.0 * hover) as u8)
        } else {
            theme::alpha(Color32::WHITE, (16.0 * hover) as u8)
        };
        painter.rect_filled(rect, 9, bg);
    }
    let color = if danger && response.hovered() {
        Color32::WHITE
    } else if response.hovered() {
        theme::TEXT
    } else {
        theme::MUTED
    };
    draw_icon(painter, rect, icon, color, 1.7);
    response
}

pub fn accent_button(
    ui: &mut Ui,
    text: &str,
    width: f32,
    height: f32,
    color: Color32,
    enabled: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id, response.hovered() && enabled);
    let painter = ui.painter();
    let base = if enabled {
        theme::mix(color, Color32::WHITE, 0.10 * hover)
    } else {
        theme::MUTED_2
    };
    if enabled {
        theme::shadow_rect(painter, rect, 14, color);
    }
    painter.rect_filled(rect, 14, base);
    if hover > 0.01 && enabled {
        painter.rect_filled(
            rect.shrink(1.0),
            13,
            theme::alpha(Color32::WHITE, (10.0 * hover) as u8),
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        theme::heading(13.0),
        if enabled { Color32::WHITE } else { theme::MUTED },
    );
    response
}

pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    let text_size = ui
        .painter()
        .layout_no_wrap(text.to_string(), theme::body(12.5), theme::TEXT)
        .size();
    let (rect, response) = ui.allocate_exact_size(vec2(text_size.x + 26.0, 36.0), Sense::click());
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        10,
        theme::alpha(Color32::WHITE, (10.0 + 10.0 * hover) as u8),
    );
    painter.rect_stroke(
        rect,
        10,
        Stroke::new(1.0, theme::mix(theme::BORDER, Color32::WHITE, 0.25 * hover)),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        theme::body(12.5),
        theme::TEXT,
    );
    response
}

pub fn text_field(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    password: bool,
) -> Response {
    ui.label(RichText::new(label).font(theme::body(10.5)).color(theme::MUTED).strong());
    ui.add_space(6.0);
    let inner = Frame::new()
        .fill(theme::alpha(Color32::WHITE, 10))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            let width = ui.available_width();
            let mut edit = TextEdit::singleline(value)
                .frame(Frame::NONE)
                .hint_text(hint)
                .font(theme::body(13.5))
                .desired_width(width);
            if password {
                edit = edit.password(true);
            }
            ui.add(edit)
        });
    inner.inner
}

pub fn text_area(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    rows: usize,
) -> Response {
    ui.label(RichText::new(label).font(theme::body(10.5)).color(theme::MUTED).strong());
    ui.add_space(6.0);
    let inner = Frame::new()
        .fill(theme::alpha(Color32::WHITE, 10))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(value)
                    .frame(Frame::NONE)
                    .hint_text(hint)
                    .font(theme::body(13.0))
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY),
            )
        });
    inner.inner
}

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::body(9.5))
            .color(theme::MUTED)
            .strong(),
    );
}

pub fn progress_bar(ui: &mut Ui, value: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4, theme::alpha(Color32::WHITE, 12));
    let value = value.clamp(0.0, 1.0);
    if value > 0.0 {
        let fill = egui::Rect::from_min_size(rect.min, vec2(rect.width() * value, rect.height()));
        theme::horizontal_gradient(painter, fill, theme::ACCENT, theme::ACCENT_2);
    }
}

pub fn avatar(
    ui: &mut Ui,
    rect: egui::Rect,
    texture: Option<TextureHandle>,
    letter: &str,
    color: Color32,
) {
    if let Some(texture) = texture {
        egui::Image::new(egui::load::SizedTexture::new(
            texture.id(),
            texture.size_vec2(),
        ))
        .uv(face_uv())
        .corner_radius(8)
        .paint_at(ui, rect);
    } else {
        let painter = ui.painter();
        painter.rect_filled(rect, 8, color);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            letter,
            theme::heading(rect.height() * 0.42),
            Color32::WHITE,
        );
    }
}

pub fn icon_circle(ui: &mut Ui, rect: egui::Rect, icon: Icon, color: Color32) {
    let painter = ui.painter();
    painter.circle_filled(rect.center(), rect.width() * 0.5, theme::alpha(color, 30));
    draw_icon(painter, rect.shrink(rect.width() * 0.27), icon, color, 1.8);
}

pub fn empty_state(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        let (rect, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
        icon_circle(ui, rect, Icon::Download, theme::ACCENT);
        ui.add_space(14.0);
        ui.label(RichText::new(title).font(theme::heading(14.0)).color(theme::TEXT));
        ui.add_space(4.0);
        ui.label(RichText::new(subtitle).font(theme::body(12.5)).color(theme::MUTED));
    });
}

pub fn modal_frame() -> Frame {
    Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(20)
        .inner_margin(Margin::symmetric(22, 20))
}

pub fn label_pill(ui: &mut Ui, text: &str, color: Color32) {
    let font = theme::body(10.0);
    let size = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
        .size();
    let (rect, _) = ui.allocate_exact_size(vec2(size.x + 18.0, 22.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 11, theme::alpha(color, 28));
    painter.rect_stroke(rect, 11, Stroke::new(1.0, theme::alpha(color, 110)), StrokeKind::Inside);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        theme::mix(color, Color32::WHITE, 0.25),
    );
}

pub fn key_value(ui: &mut Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label)
                .font(theme::body(9.0))
                .color(theme::MUTED)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(RichText::new(value).font(theme::body(13.0)).color(theme::TEXT).strong());
    });
}

pub fn small_spinner(ui: &mut Ui, t: f64) {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let radius = 7.0;
    let start = t as f32 * 4.0;
    let mut prev = center + vec2(start.cos(), start.sin()) * radius;
    for i in 1..=18 {
        let a = start + i as f32 / 18.0 * 4.4;
        let p = center + vec2(a.cos(), a.sin()) * radius;
        let alpha = (i as f32 / 18.0 * 220.0) as u8;
        painter.line_segment(
            [prev, p],
            Stroke::new(2.0, theme::alpha(theme::ACCENT_2, alpha)),
        );
        prev = p;
    }
}

pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0, theme::alpha(Color32::WHITE, 14));
}

pub fn scroll_style() -> egui::style::ScrollStyle {
    let mut scroll = egui::style::ScrollStyle::solid();
    scroll.bar_width = 8.0;
    scroll.bar_inner_margin = 2.0;
    scroll
}

pub fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font, Color32::WHITE)
        .size()
        .x
}

pub fn spacer(ui: &mut Ui, height: f32) {
    ui.add_space(height);
}

pub fn line(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [pos2(rect.left(), rect.center().y), pos2(rect.right(), rect.center().y)],
        Stroke::new(1.0, color),
    );
}

pub fn icon_text_button(ui: &mut Ui, text: &str, icon: Icon, color: Color32) -> Response {
    let font = theme::body(12.5);
    let text_w = text_width(ui, text, font.clone());
    let (rect, response) = ui.allocate_exact_size(vec2(text_w + 46.0, 36.0), Sense::click());
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        10,
        theme::alpha(Color32::WHITE, (8.0 + 10.0 * hover) as u8),
    );
    draw_icon(
        painter,
        egui::Rect::from_center_size(rect.left_center() + vec2(16.0, 0.0), vec2(16.0, 16.0)),
        icon,
        color,
        1.7,
    );
    painter.text(
        rect.left_center() + vec2(30.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        font,
        theme::TEXT,
    );
    response
}

pub fn full_width_button(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let width = ui.available_width();
    accent_button(ui, text, width, 42.0, color, true)
}

pub fn tiny_badge(ui: &mut Ui, text: &str, color: Color32) {
    let font = theme::body(9.0);
    let size = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
        .size();
    let (rect, _) = ui.allocate_exact_size(vec2(size.x + 14.0, 18.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 9, theme::alpha(color, 24));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        color,
    );
}

pub fn right_aligned(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add);
}

pub fn wrap_text(ui: &mut Ui, text: &str, color: Color32, size: f32) {
    ui.label(RichText::new(text).font(theme::body(size)).color(color));
}

pub fn card_rect(ui: &mut Ui, size: Vec2, sense: Sense) -> (egui::Rect, Response) {
    ui.allocate_exact_size(size, sense)
}
