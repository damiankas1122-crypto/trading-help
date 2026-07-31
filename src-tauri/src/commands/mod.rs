//! Tauri commands exposed to the frontend (IPC contract: Result<T, String>).
//! Every command is a thin wrapper: it calls the matching `*_inner` submodule
//! (Result<T, CommandError>) and stringifies the error in this one place. No
//! business logic lives here.

use crate::{models, history_store, keychain};
use tauri::AppHandle;

mod error;
mod instruments;
mod cross_market;
mod precious_metals;
mod custom_pair;
mod tactics;
mod market_context;
mod instrument_briefing;
mod live_quotes;
pub use error::CommandError;

#[tauri::command]
pub async fn get_cross_market_analysis() -> Result<Vec<models::AnalyticalReport>, String> {
    cross_market::get_cross_market_analysis_inner()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_precious_metals_analysis() -> Result<models::PreciousMetalsReport, String> {
    precious_metals::get_precious_metals_analysis_inner()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_custom_pair_correlation(ticker_a: String, ticker_b: String) -> Result<models::AnalyticalReport, String> {
    custom_pair::get_custom_pair_correlation_inner(ticker_a, ticker_b)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_live_quotes(instruments: Vec<String>) -> Result<Vec<models::LiveQuote>, String> {
    live_quotes::get_live_quotes_inner(instruments)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_market_context(app: AppHandle) -> Result<models::MarketContext, String> {
    market_context::get_market_context_inner(app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_instrument_briefing(
    instrument: String,
    operation_id: String,
) -> Result<models::InstrumentBriefing, String> {
    instrument_briefing::get_instrument_briefing_inner(instrument, operation_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_trading_tactic(
    app: AppHandle,
    instrument: String,
    operation_id: String,
) -> Result<models::TradingTactic, String> {
    tactics::generate_trading_tactic_inner(&app, instrument, operation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Cancels an in-flight AI call. `false` means there was nothing to cancel -
/// an ordinary outcome after a reopened window or a call that just finished,
/// not an error worth surfacing.
#[tauri::command]
pub fn cancel_operation(operation_id: String) -> bool {
    crate::ai_engine::cancel::cancel(&operation_id)
}

#[tauri::command]
pub async fn get_tactic_track_record(app: AppHandle) -> Result<models::TacticTrackRecord, String> {
    tactics::get_tactic_track_record_inner(&app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_last_snapshot(app: AppHandle) -> Option<models::Snapshot> {
    history_store::load_last_snapshot(&app)
}

#[tauri::command]
pub fn save_gemini_api_key(key: String) -> Result<(), String> {
    keychain::save_gemini_api_key(&key).map_err(|e| CommandError::Keychain(e).to_string())
}

#[tauri::command]
pub fn has_gemini_api_key() -> bool {
    keychain::has_gemini_api_key()
}

#[tauri::command]
pub fn delete_gemini_api_key() -> Result<(), String> {
    keychain::delete_gemini_api_key().map_err(|e| CommandError::Keychain(e).to_string())
}

/// Opens the log directory in the system file manager. The path comes from
/// Tauri, never from the frontend, so there is nothing for a caller to point
/// elsewhere.
#[tauri::command]
pub fn open_log_directory(app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_opener::OpenerExt;

    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Nie udało się ustalić katalogu z logami: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Nie udało się utworzyć katalogu z logami: {e}"))?;

    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Nie udało się otworzyć katalogu z logami: {e}"))
}
