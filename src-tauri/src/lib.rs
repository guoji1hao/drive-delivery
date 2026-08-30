mod adapters;
mod auth;
mod clipboard;
mod commands;
mod database;
mod errors;
mod logging;
mod models;
mod secret_store;
mod settings;
mod share_manager;
mod state;
mod task_manager;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AppState::initialize(app.handle().clone())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize_app,
            commands::save_settings,
            commands::save_baidu_credentials,
            commands::start_baidu_auth,
            commands::poll_baidu_auth,
            commands::clear_baidu_auth,
            commands::enqueue_paths,
            commands::pause_task,
            commands::cancel_task,
            commands::retry_task,
            commands::search_history,
            commands::copy_task_result,
            commands::copy_history_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
