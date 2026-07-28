//! Dane liczbowe rynku (korelacje, GSR, szablonowe Pine Scripty) - bez AI,
//! bez rate-limitu. Odświeżane niezależnie od briefingu pojedynczego
//! instrumentu (patrz `instrument_briefing.rs`), zasila ticker tape i panel
//! "Kontekst rynkowy". Zero #[tauri::command] tutaj.

use crate::{models, history_store, ai_engine};
use tauri::AppHandle;
use time::OffsetDateTime;
use super::cross_market;
use super::precious_metals;
use super::error::CommandError;

pub(crate) async fn get_market_context_inner(app: AppHandle) -> Result<models::MarketContext, CommandError> {
    let equity_reports = cross_market::get_cross_market_analysis_inner().await?;
    let metals_report = precious_metals::get_precious_metals_analysis_inner().await?;

    let strongest_equity = ai_engine::find_strongest_equity_pair(&equity_reports)
        .ok_or(CommandError::NoStrongestPair)?;

    let pine_script_correlation = ai_engine::generate_correlation_pine_script(&strongest_equity.symbol);
    let pine_script_correlation_explanation = ai_engine::explain_correlation_script(&strongest_equity.symbol);
    let pine_script_gsr = ai_engine::generate_gsr_pine_script();
    let pine_script_gsr_explanation = ai_engine::explain_gsr_script();

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();

    let snapshot = models::Snapshot {
        equity_reports: equity_reports.clone(),
        metals_report: metals_report.clone(),
        timestamp: timestamp.clone(),
    };
    history_store::save_snapshot(&app, &snapshot).map_err(CommandError::Storage)?;

    Ok(models::MarketContext {
        equity_reports,
        metals_report,
        pine_script_correlation,
        pine_script_correlation_explanation,
        pine_script_gsr,
        pine_script_gsr_explanation,
        timestamp,
    })
}
