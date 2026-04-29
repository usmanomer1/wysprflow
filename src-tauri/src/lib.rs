use tauri::Manager;
use tracing_subscriber::EnvFilter;

mod app_context;
mod audio;
mod ax;
mod commands;
mod db;
mod dictionary;
mod history;
mod hotkey;
mod hud;
mod llm;
mod pipeline;
mod settings;
mod snippets;
mod startup;
mod stt;
mod vad;

use crate::pipeline::Pipeline;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,wysprflow=debug")),
        )
        .with_target(false)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            settings::initialize(app.handle())?;
            startup::set_launch_at_login(settings::get().launch_at_login).ok();

            // SQLite for dictionary, snippets, history.
            let dir = app
                .path()
                .app_config_dir()
                .map_err(|e| anyhow::anyhow!("app_config_dir: {}", e))?;
            let db_path = dir.join("wysprflow.db");
            let db = db::open(&db_path)?;
            app.manage(db);

            // Pipeline owns the dictation state machine and reads dict / writes history.
            let pipeline = Pipeline::new(app.handle().clone());
            app.manage(pipeline);

            hotkey::initialize(app.handle())?;
            hud::initialize(app.handle())?;
            tracing::info!("wysprflow ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::update_config,
            commands::get_api_key_status,
            commands::list_api_key_statuses,
            commands::set_api_key,
            commands::delete_api_key,
            commands::validate_api_key,
            commands::get_installation_status,
            commands::list_audio_input_devices,
            commands::set_hotkey,
            commands::check_permissions,
            commands::request_microphone,
            commands::move_to_applications,
            commands::open_accessibility_settings,
            commands::open_input_monitoring_settings,
            commands::request_accessibility,
            commands::open_microphone_settings,
            commands::start_dictation,
            commands::stop_dictation,
            commands::list_dictionary,
            commands::add_dictionary_word,
            commands::delete_dictionary_word,
            commands::toggle_dictionary_star,
            commands::list_snippets,
            commands::upsert_snippet,
            commands::delete_snippet,
            commands::list_history,
            commands::search_history,
            commands::delete_history_entry,
            commands::clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
