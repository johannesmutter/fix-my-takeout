mod commands;
mod db;
mod fs;
mod metadata;
mod pipeline;
mod progress;

use commands::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            db: Mutex::new(None),
            tracker: Mutex::new(None),
            source_path: Mutex::new(None),
            output_path: Mutex::new(None),
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    use tauri::window::Effect;
                    let _ = window.set_effects(tauri::window::EffectsBuilder::new()
                        .effect(Effect::HudWindow)
                        .build());
                }
            }
            Ok(())
        })
        .menu(|handle| {
            use tauri::menu::*;

            let app_menu = SubmenuBuilder::new(handle, "Fix My Takeout")
                .about(None)
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let file_menu = SubmenuBuilder::new(handle, "File")
                .item(&MenuItemBuilder::with_id("new_export", "New Export").accelerator("CmdOrCtrl+N").build(handle)?)
                .separator()
                .close_window()
                .build()?;

            let edit_menu = SubmenuBuilder::new(handle, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let window_menu = SubmenuBuilder::new(handle, "Window")
                .minimize()
                .maximize()
                .separator()
                .fullscreen()
                .build()?;

            let menu = MenuBuilder::new(handle)
                .item(&app_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&window_menu)
                .build()?;

            Ok(menu)
        })
        .on_menu_event(|app, event| {
            if event.id() == "new_export" {
                use tauri::{Emitter, Manager};
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("menu-new-export", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_processing,
            commands::pause_processing,
            commands::resume_processing,
            commands::cancel_processing,
            commands::get_zip_statuses,
            commands::get_disk_info,
            commands::check_existing_session,
            commands::get_summary_stats,
            commands::open_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
