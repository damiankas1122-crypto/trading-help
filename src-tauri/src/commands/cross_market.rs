//! Korelacja equity (NASDAQ<->SP500): fetch danych, korelacja z laggiem,
//! zmienność i wskaźniki techniczne per para leader/follower. Matematyka
//! korelacji (`calculate_correlation`, `to_returns`, `align_and_correlate_lagged`)
//! żyje tutaj, ale jest współdzielona z `precious_metals.rs` (korelacja
//! Au-Ag) - stąd `pub(crate)`, nie prywatne. `numeric_context_for_equity`
//! współdzielone z `briefing.rs`/`tactics.rs`. Zero #[tauri::command]
//! tutaj - te żyją w commands/mod.rs.

use crate::{models, market_engine, analysis_engine};
use time::OffsetDateTime;
use super::error::CommandError;

pub(crate) fn calculate_correlation(data_a: &[f64], data_b: &[f64]) -> f64 {
    if data_a.is_empty() || data_b.is_empty() || data_a.len() != data_b.len() {
        return 0.0;
    }

    let n = data_a.len() as f64;
    let mean_a = data_a.iter().sum::<f64>() / n;
    let mean_b = data_b.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut sum_sq_a = 0.0;
    let mut sum_sq_b = 0.0;

    for i in 0..data_a.len() {
        let diff_a = data_a[i] - mean_a;
        let diff_b = data_b[i] - mean_b;
        numerator += diff_a * diff_b;
        sum_sq_a += diff_a * diff_a;
        sum_sq_b += diff_b * diff_b;
    }

    let denominator = (sum_sq_a * sum_sq_b).sqrt();
    if denominator == 0.0 {
        return 0.0;
    }
    numerator / denominator
}

pub(crate) fn to_returns(closes: &[f64]) -> Vec<f64> {
    closes
        .windows(2)
        .filter(|w| w[0] != 0.0)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect()
}

pub(crate) fn align_and_correlate_lagged(leader: &[f64], follower: &[f64], lag: usize) -> f64 {
    let len = leader.len().min(follower.len());
    if len <= lag + 1 {
        return 0.0;
    }
    let leader_tail = &leader[leader.len() - len..];
    let follower_tail = &follower[follower.len() - len..];
    calculate_correlation(&leader_tail[..len - lag], &follower_tail[lag..])
}

const DEFAULT_LAG: usize = 1;

pub(crate) fn daily_change_pct(closes: &[f64]) -> f64 {
    if closes.len() < 2 {
        return 0.0;
    }
    let prev = closes[closes.len() - 2];
    let last = closes[closes.len() - 1];
    if prev == 0.0 {
        return 0.0;
    }
    ((last - prev) / prev) * 100.0
}

/// jeden AnalyticalReport dla pary leader/follower - wydzielone, żeby dało
/// się testować bez sieci
fn build_pair_report(
    leader_label: &str,
    leader_data: &[models::MarketData],
    leader_closes: &[f64],
    follower_label: &str,
    follower_closes: &[f64],
    timestamp: &str,
) -> models::AnalyticalReport {
    let leader_returns = to_returns(leader_closes);
    let follower_returns = to_returns(follower_closes);
    let correlation = align_and_correlate_lagged(&leader_returns, &follower_returns, DEFAULT_LAG);
    // zmienność z leadera, nie followera - kiedyś to się pomyliło i był z tego bug
    let volatility = analysis_engine::calculate_volatility(leader_data);
    // RSI/MACD też z leadera - follower to tylko punkt odniesienia
    let technicals = analysis_engine::calculate_technicals(leader_data);
    let latest_close = leader_closes.last().copied().unwrap_or(0.0);
    let daily_change = daily_change_pct(leader_closes);

    models::AnalyticalReport {
        symbol: format!("{}->{}", leader_label, follower_label),
        correlation,
        volatility,
        technicals,
        latest_close,
        daily_change_pct: daily_change,
        timestamp: timestamp.to_string(),
    }
}

pub(crate) async fn get_cross_market_analysis_inner() -> Result<Vec<models::AnalyticalReport>, CommandError> {
    let (nasdaq, sp500) = tokio::join!(
        market_engine::fetch_market_data("^IXIC"),
        market_engine::fetch_market_data("^GSPC"),
    );

    let nasdaq = nasdaq.map_err(CommandError::MarketData)?;
    let sp500 = sp500.map_err(CommandError::MarketData)?;

    let markets: Vec<(&str, &Vec<models::MarketData>, Vec<f64>)> = vec![
        ("NASDAQ", &nasdaq, nasdaq.iter().map(|d| d.close).collect()),
        ("SP500", &sp500, sp500.iter().map(|d| d.close).collect()),
    ];

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
    let mut reports = Vec::new();

    for (leader_label, leader_data, leader_closes) in &markets {
        for (follower_label, _follower_data, follower_closes) in &markets {
            if leader_label == follower_label {
                continue;
            }
            reports.push(build_pair_report(
                leader_label,
                leader_data,
                leader_closes,
                follower_label,
                follower_closes,
                &timestamp,
            ));
        }
    }

    Ok(reports)
}

pub(crate) fn numeric_context_for_equity(instrument: &str, reports: &[models::AnalyticalReport]) -> String {
    let leader_prefix = format!("{}->", instrument);
    reports
        .iter()
        .filter(|r| r.symbol.starts_with(&leader_prefix))
        .map(|r| format!(
            "- {}: korelacja={:.4}, zmienność={:.4}, RSI(14)={:.2}, MACD={:.4} (sygnał={:.4})",
            r.symbol, r.correlation, r.volatility, r.technicals.rsi, r.technicals.macd_line, r.technicals.macd_signal
        ))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod build_pair_report_tests {
    use super::*;

    fn candle(close: f64) -> models::MarketData {
        models::MarketData {
            symbol: "TEST".to_string(),
            time: "2026-01-01".to_string(),
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
        }
    }

    #[test]
    fn volatility_comes_from_leader_not_follower() {
        let leader_data = vec![candle(100.0), candle(110.0), candle(90.0), candle(105.0)];
        let follower_data = vec![candle(100.0), candle(100.5), candle(99.5), candle(100.0)];

        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();
        let follower_closes: Vec<f64> = follower_data.iter().map(|d| d.close).collect();

        let report = build_pair_report(
            "NASDAQ",
            &leader_data,
            &leader_closes,
            "SP500",
            &follower_closes,
            "2026-01-01",
        );

        let expected_leader_volatility = analysis_engine::calculate_volatility(&leader_data);
        let follower_volatility = analysis_engine::calculate_volatility(&follower_data);

        assert!((report.volatility - expected_leader_volatility).abs() < 1e-9);
        assert!((report.volatility - follower_volatility).abs() > 1e-9);
    }

    #[test]
    fn symbol_format_is_leader_arrow_follower() {
        let data = vec![candle(100.0), candle(101.0)];
        let closes: Vec<f64> = data.iter().map(|d| d.close).collect();

        let report = build_pair_report("GOLD", &data, &closes, "SILVER", &closes, "2026-01-01");
        assert_eq!(report.symbol, "GOLD->SILVER");
    }

    #[test]
    fn latest_close_is_leaders_last_candle() {
        let leader_data = vec![candle(100.0), candle(110.0), candle(105.0)];
        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();
        let follower_data = vec![candle(50.0), candle(51.0), candle(52.0)];
        let follower_closes: Vec<f64> = follower_data.iter().map(|d| d.close).collect();

        let report = build_pair_report(
            "NASDAQ", &leader_data, &leader_closes, "SP500", &follower_closes, "2026-01-01",
        );

        assert_eq!(report.latest_close, 105.0);
    }

    #[test]
    fn daily_change_pct_computed_from_leaders_last_two_candles() {
        let leader_data = vec![candle(100.0), candle(110.0)];
        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();
        let follower_data = vec![candle(50.0), candle(50.0)];
        let follower_closes: Vec<f64> = follower_data.iter().map(|d| d.close).collect();

        let report = build_pair_report(
            "NASDAQ", &leader_data, &leader_closes, "SP500", &follower_closes, "2026-01-01",
        );

        assert!((report.daily_change_pct - 10.0).abs() < 1e-9);
    }

    #[test]
    fn daily_change_pct_is_zero_for_single_candle() {
        assert_eq!(daily_change_pct(&[100.0]), 0.0);
        assert_eq!(daily_change_pct(&[]), 0.0);
    }

    #[test]
    fn daily_change_pct_ignores_zero_previous_close() {
        assert_eq!(daily_change_pct(&[0.0, 100.0]), 0.0);
    }
}
