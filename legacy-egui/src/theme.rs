use eframe::egui;
use egui::{
    epaint::{Mesh, Vertex, WHITE_UV},
    vec2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke,
};
use std::f32::consts::TAU;
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(7, 7, 14);
pub const BG_2: Color32 = Color32::from_rgb(10, 10, 20);
pub const PANEL: Color32 = Color32::from_rgb(17, 17, 31);
pub const PANEL_2: Color32 = Color32::from_rgb(22, 22, 40);
pub const CARD: Color32 = Color32::from_rgb(18, 18, 33);
pub const BORDER: Color32 = Color32::from_rgb(45, 45, 70);
pub const TEXT: Color32 = Color32::from_rgb(238, 240, 255);
pub const MUTED: Color32 = Color32::from_rgb(145, 150, 182);
pub const MUTED_2: Color32 = Color32::from_rgb(100, 105, 135);
pub const ACCENT: Color32 = Color32::from_rgb(139, 92, 246);
pub const ACCENT_2: Color32 = Color32::from_rgb(34, 211, 238);
pub const PINK: Color32 = Color32::from_rgb(244, 114, 182);
pub const OK: Color32 = Color32::from_rgb(52, 211, 153);
pub const WARN: Color32 = Color32::from_rgb(251, 191, 36);
pub const DANGER: Color32 = Color32::from_rgb(251, 113, 133);
pub const ORANGE: Color32 = Color32::from_rgb(251, 146, 60);

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "golos".to_string(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/GolosText.ttf"
        ))),
    );
    fonts.font_data.insert(
        "unbounded".to_string(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Unbounded.ttf"
        ))),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "golos".to_string());
    fonts.families.insert(
        FontFamily::Name("Unbounded".into()),
        vec!["unbounded".to_string(), "golos".to_string()],
    );
    ctx.set_fonts(fonts);

    ctx.set_theme(egui::ThemePreference::Dark);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = PANEL;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.extreme_bg_color = BG_2;
    visuals.faint_bg_color = Color32::from_rgba_premultiplied(255, 255, 255, 4);
    visuals.code_bg_color = PANEL_2;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.55);
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.hyperlink_color = ACCENT_2;
    visuals.override_text_color = Some(TEXT);
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgba_premultiplied(255, 255, 255, 8);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = Color32::from_rgba_premultiplied(255, 255, 255, 16);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.5);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.open.bg_fill = PANEL_2;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.visuals = visuals;
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.spacing.button_padding = vec2(12.0, 8.0);
        style.spacing.interact_size.y = 34.0;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.floating = false;
        style.text_styles.insert(
            egui::TextStyle::Body,
            FontId::new(14.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            FontId::new(20.0, unbounded_family()),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        );
    });
}

pub fn unbounded_family() -> FontFamily {
    FontFamily::Name("Unbounded".into())
}

pub fn heading(size: f32) -> FontId {
    FontId::new(size, unbounded_family())
}

pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn alpha(color: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

pub fn rgb(tuple: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(tuple.0, tuple.1, tuple.2)
}

pub fn radial_glow(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: center,
        uv: WHITE_UV,
        color,
    });
    let segments = 56;
    for i in 0..=segments {
        let angle = i as f32 / segments as f32 * TAU;
        mesh.vertices.push(Vertex {
            pos: center + vec2(angle.cos(), angle.sin()) * radius,
            uv: WHITE_UV,
            color: Color32::TRANSPARENT,
        });
    }
    for i in 0..segments {
        mesh.add_triangle(0, i + 1, i + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}

pub fn vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: WHITE_UV,
        color: bottom,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: WHITE_UV,
        color: bottom,
    });
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn horizontal_gradient(painter: &Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: WHITE_UV,
        color: left,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: WHITE_UV,
        color: right,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: WHITE_UV,
        color: right,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: WHITE_UV,
        color: left,
    });
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn hexagon(center: Pos2, radius: f32) -> Vec<Pos2> {
    (0..6)
        .map(|i| {
            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * TAU / 6.0;
            center + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

pub fn draw_logo(painter: &Painter, rect: Rect, t: f32) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.46;
    let points = hexagon(center, radius);
    painter.add(egui::Shape::convex_polygon(
        points.clone(),
        Color32::from_rgba_unmultiplied(139, 92, 246, 36),
        Stroke::new(1.6, ACCENT.gamma_multiply(0.9)),
    ));
    let pulse = (t * 1.8).sin() * 0.5 + 0.5;
    let inner = hexagon(center, radius * 0.58);
    painter.add(egui::Shape::closed_line(
        inner,
        Stroke::new(1.2, mix(ACCENT, ACCENT_2, pulse).gamma_multiply(0.75)),
    ));
    let font = heading(radius * 0.95);
    painter.text(
        center + vec2(0.0, -1.0),
        egui::Align2::CENTER_CENTER,
        "T",
        font,
        Color32::WHITE,
    );
}

pub fn shadow_rect(painter: &Painter, rect: Rect, radius: u8, color: Color32) {
    for i in 1..=6 {
        let grow = i as f32 * 2.0;
        let a = (14u8).saturating_sub(i as u8 * 2);
        painter.rect_filled(
            rect.expand(grow),
            radius.saturating_add(i as u8),
            alpha(color, a),
        );
    }
}

pub fn badge_color(kind: &str) -> (Color32, Color32) {
    match kind {
        "ok" => (OK, Color32::from_rgba_unmultiplied(52, 211, 153, 26)),
        "update" => (WARN, Color32::from_rgba_unmultiplied(251, 191, 36, 26)),
        "new" => (ACCENT_2, Color32::from_rgba_unmultiplied(34, 211, 238, 26)),
        _ => (MUTED, Color32::from_rgba_unmultiplied(255, 255, 255, 10)),
    }
}


