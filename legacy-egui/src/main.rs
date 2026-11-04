#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod auth;
mod events;
mod game;
mod images;
mod models;
mod net;
mod publish;
mod store;
mod theme;
mod util;
#[allow(dead_code)]
mod widgets;

use eframe::egui;
use std::sync::Arc;

fn main() -> eframe::Result {
    let store = store::Store::new();
    let width = store.settings.launcher_width.max(980.0);
    let height = store.settings.launcher_height.max(640.0);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TDEV Launcher")
            .with_inner_size([width, height])
            .with_min_inner_size([980.0, 640.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_icon(Arc::new(make_icon())),
        centered: true,
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "TDEV Launcher",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, store)))),
    )
}

fn make_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let fx = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let fy = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let i = (y * size + x) * 4;
            // тёмный фон
            let mut r = 8.0;
            let mut g = 8.0;
            let mut b = 16.0;
            // мягкое свечение
            let glow = (1.0 - (fx * fx + fy * fy).sqrt()).max(0.0);
            r += 40.0 * glow;
            g += 18.0 * glow;
            b += 90.0 * glow;
            // шестиугольник
            let hex = (fx.abs() * 0.866 + fy.abs() * 0.5).max(fy.abs());
            let inside = hex < 0.72;
            let edge = (hex - 0.72).abs() < 0.045;
            if inside {
                r = r * 0.55 + 35.0;
                g = g * 0.55 + 24.0;
                b = b * 0.55 + 90.0;
            }
            if edge {
                let t = (fx + 1.0) * 0.5;
                r = 139.0 * (1.0 - t) + 34.0 * t;
                g = 92.0 * (1.0 - t) + 211.0 * t;
                b = 246.0 * (1.0 - t) + 238.0 * t;
            }
            // буква T
            let t_vertical = fx.abs() < 0.065 && fy > -0.26 && fy < 0.34;
            let t_horizontal = fy.abs() < 0.065 && fx.abs() < 0.30 && fy < -0.14;
            if (t_vertical || t_horizontal) && inside {
                r = 245.0;
                g = 247.0;
                b = 255.0;
            }
            rgba[i] = r.clamp(0.0, 255.0) as u8;
            rgba[i + 1] = g.clamp(0.0, 255.0) as u8;
            rgba[i + 2] = b.clamp(0.0, 255.0) as u8;
            rgba[i + 3] = 255;
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
