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
