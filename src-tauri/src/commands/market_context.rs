//! Numeric market data (correlations, GSR, templated Pine Scripts): no AI, no
//! rate limit. Refreshed independently of per-instrument briefings (see
//! `instrument_briefing.rs`); feeds the ticker tape and the market context
//! panel. No #[tauri::command] here.

use crate::{models, history_store, ai_engine, catalog};
use tauri::AppHandle;
use time::OffsetDateTime;
use super::cross_market;
use super::precious_metals;
use super::error::CommandError;

/// Splits a "LEADER->FOLLOWER" report symbol back into catalogue entries.
fn resolve_pair(pair_symbol: &str) -> Option<(&'static catalog::Instrument, &'static catalog::Instrument)> {
    let (leader, follower) = pair_symbol.split_once("->")?;
    Some((catalog::find(leader.trim())?, catalog::find(follower.trim())?))
}

pub(crate) async fn get_market_context_inner(app: AppHandle) -> Result<models::MarketContext, CommandError> {
    let equity_reports = cross_market::get_cross_market_analysis_inner().await?;
    let metals_report = precious_metals::get_precious_metals_analysis_inner().await?;

    let strongest_equity = ai_engine::find_strongest_equity_pair(&equity_reports)
        .ok_or(CommandError::NoStrongestPair)?;

    // Both sides must be catalogued before a script is emitted; an unresolvable
    // pair used to render as NASDAQ/SP500, handing the user a correct-looking
    // script about a market they did not ask about.
    let (leader, follower) = resolve_pair(&strongest_equity.symbol)
        .ok_or_else(|| CommandError::UnknownInstrument(strongest_equity.symbol.clone()))?;

    let pine_script_correlation = ai_engine::generate_correlation_pine_script(leader, follower);
    let pine_script_correlation_explanation = ai_engine::explain_correlation_script(leader, follower);
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
