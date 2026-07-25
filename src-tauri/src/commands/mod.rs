//! Komendy Tauri eksponowane do frontendu (kontrakt IPC: Result<T, String>).
//! Każda komenda to cienki wrapper: woła *_inner z odpowiedniego submodułu
//! (Result<T, CommandError>) i stringifikuje błąd w jednym miejscu, na
//! samym końcu - żadna logika biznesowa nie mieszka bezpośrednio tutaj.

use crate::{models, history_store, keychain};
use tauri::AppHandle;

mod error;
mod cross_market;
mod precious_metals;
mod tactics;
mod briefing;
pub use error::CommandError;

#[tauri::command]
pub fn calculate_correlation(data_a: Vec<f64>, data_b: Vec<f64>) -> f64 {
    cross_market::calculate_correlation(data_a, data_b)
}

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
pub async fn get_full_briefing(app: AppHandle, slot: String) -> Result<models::FullBriefing, String> {
    briefing::get_full_briefing_inner(app, slot)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_trading_tactic(app: AppHandle, instrument: String) -> Result<models::TradingTactic, String> {
    tactics::generate_trading_tactic_inner(&app, instrument)
        .await
        .map_err(|e| e.to_string())
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
