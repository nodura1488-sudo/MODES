#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod events;
mod game;
mod models;
mod net;
mod state;
mod store;
mod update;
mod util;

use state::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::win_minimize,
            commands::win_close,
            commands::win_splash,
            commands::win_ready,
            commands::modes_get,
            commands::modes_refresh,
            commands::state_get,
            commands::settings_get,
            commands::settings_save,
            commands::settings_choose_dir,
            commands::shell_open,
            commands::game_open_folder,
            commands::app_data_path,
            commands::app_open_folder,
            commands::accounts_get,
            commands::accounts_current,
            commands::accounts_device_start,
            commands::accounts_device_poll,
            commands::accounts_select,
            commands::accounts_remove,
            commands::accounts_refresh,
            commands::open_ely_window,
            commands::close_ely_window,
            commands::game_install,
            commands::game_play,
            commands::game_cancel,
            commands::game_uninstall,
            commands::game_set_library,
            commands::game_open_mode_folder,
            update::check_update,
            update::download_update,
            update::install_update,
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        if let Some(win) = app_handle.get_webview_window("main") {
                            commands::save_window_size_for_event(&win, &app_handle);
                        }
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
