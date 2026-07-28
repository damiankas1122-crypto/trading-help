//! Precious metals analysis: Au-Ag correlation, Gold/Silver Ratio (current and
//! 30 days back), volatility and technical indicators for both metals. Uses the
//! correlation maths from `cross_market.rs` rather than keeping its own copy.
//! No #[tauri::command] here.

use crate::{models, market_engine, analysis_engine};
use time::OffsetDateTime;
use super::cross_market::{to_returns, align_and_correlate_lagged, daily_change_pct};
use super::error::CommandError;

/// Close of the candle nearest `target_unix`. Since the window grew to 90 days,
/// the first candle can no longer be assumed to be "30 days ago".
fn close_nearest_to(data: &[models::MarketData], target_unix: i64) -> Option<f64> {
    data.iter()
        .min_by_key(|d| (d.timestamp - target_unix).abs())
        .map(|d| d.close)
}

pub(crate) async fn get_precious_metals_analysis_inner() -> Result<models::PreciousMetalsReport, CommandError> {
    let (gold, silver) = tokio::join!(
        market_engine::fetch_market_data("GC=F"),
        market_engine::fetch_market_data("SI=F"),
    );

    let gold = gold.map_err(CommandError::MarketData)?;
    let silver = silver.map_err(CommandError::MarketData)?;

    let gold_closes: Vec<f64> = gold.iter().map(|d| d.close).collect();
    let silver_closes: Vec<f64> = silver.iter().map(|d| d.close).collect();

    let gold_returns = to_returns(&gold_closes);
    let silver_returns = to_returns(&silver_closes);
    let correlation = align_and_correlate_lagged(&gold_returns, &silver_returns, 0);

    let gold_volatility = analysis_engine::calculate_volatility(&gold);
    let silver_volatility = analysis_engine::calculate_volatility(&silver);
    let gold_technicals = analysis_engine::calculate_technicals(&gold);
    let silver_technicals = analysis_engine::calculate_technicals(&silver);

    let current_gsr = match (gold_closes.last(), silver_closes.last()) {
        (Some(g), Some(s)) if *s != 0.0 => g / s,
        _ => 0.0,
    };
    let thirty_days_ago_unix = (OffsetDateTime::now_utc() - time::Duration::days(30)).unix_timestamp();
    let gsr_30d_ago = match (
        close_nearest_to(&gold, thirty_days_ago_unix),
        close_nearest_to(&silver, thirty_days_ago_unix),
    ) {
        (Some(g), Some(s)) if s != 0.0 => g / s,
        _ => 0.0,
    };
    let gsr_change_pct = if gsr_30d_ago != 0.0 {
        ((current_gsr - gsr_30d_ago) / gsr_30d_ago) * 100.0
    } else {
        0.0
    };

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
    let gold_price = gold_closes.last().copied().unwrap_or(0.0);
    let silver_price = silver_closes.last().copied().unwrap_or(0.0);

    Ok(models::PreciousMetalsReport {
        correlation,
        current_gsr,
        gsr_30d_ago,
        gsr_change_pct,
        gold_volatility,
        silver_volatility,
        gold_technicals,
        silver_technicals,
        gold_price,
        silver_price,
        gold_daily_change_pct: daily_change_pct(&gold_closes),
        silver_daily_change_pct: daily_change_pct(&silver_closes),
        timestamp,
    })
}

pub(crate) fn numeric_context_for_metal(metal: &str, report: &models::PreciousMetalsReport) -> String {
    let (volatility, technicals) = if metal == "GOLD" {
        (report.gold_volatility, &report.gold_technicals)
    } else {
        (report.silver_volatility, &report.silver_technicals)
    };
    format!(
        "- Korelacja Złoto-Srebro: {:.4}\n- Obecny GSR: {:.2}\n- GSR 30 dni temu: {:.2}\n- Zmiana GSR: {:.2}%\n- Zmienność {}: {:.4}\n- RSI(14) {}: {:.2}\n- MACD {}: {:.4} (sygnał={:.4})",
        report.correlation, report.current_gsr, report.gsr_30d_ago, report.gsr_change_pct,
        metal, volatility, metal, technicals.rsi, metal, technicals.macd_line, technicals.macd_signal
    )
}
