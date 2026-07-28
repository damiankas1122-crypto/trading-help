// src-tauri/src/lib.rs
pub mod models;
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
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
             commands::get_cross_market_analysis,
             commands::get_precious_metals_analysis,
             commands::get_custom_pair_correlation,
             commands::get_market_context,
             commands::get_instrument_briefing,
             commands::generate_trading_tactic,
             commands::get_tactic_track_record,
             commands::get_last_snapshot,
             commands::save_gemini_api_key,
             commands::has_gemini_api_key,
             commands::delete_gemini_api_key
        ])
        .run(tauri::generate_context!())
        .expect("Błąd uruchamiania aplikacji");
}
