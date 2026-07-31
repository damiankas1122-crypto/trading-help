// src-tauri/src/lib.rs
pub mod models;
pub mod catalog;
pub mod logging;
pub mod market_engine;
pub mod analysis_engine;
pub mod ai_engine;
pub mod news_engine;
pub mod history_store;
pub mod tactic_engine;
pub mod tactic_store;
pub mod commands;
pub mod keychain;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;

            // A missing or unusable log directory degrades to no log file; it
            // never blocks startup.
            let log_path = app.path().app_log_dir().ok().and_then(|dir| logging::init(&dir));

            logging::log_startup(
                env!("CARGO_PKG_VERSION"),
                ai_engine::GEMINI_MODEL,
                tactic_store::load_all(app.handle()).len(),
                keychain::has_gemini_api_key(),
            );
            if log_path.is_none() {
                log::warn!("Log directory unavailable; running without a log file");
            }
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
             commands::get_cross_market_analysis,
             commands::get_precious_metals_analysis,
             commands::get_custom_pair_correlation,
             commands::get_live_quotes,
             commands::get_market_context,
             commands::get_instrument_briefing,
             commands::generate_trading_tactic,
             commands::get_tactic_track_record,
             commands::get_last_snapshot,
             commands::save_gemini_api_key,
             commands::has_gemini_api_key,
             commands::delete_gemini_api_key,
             commands::open_log_directory,
             commands::cancel_operation
        ])
        .run(tauri::generate_context!())
        .expect("failed to start application");
}
