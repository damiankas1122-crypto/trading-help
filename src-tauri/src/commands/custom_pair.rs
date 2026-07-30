//! Arbitrary correlation pair (X vs Y), independent of the watchlist. Reuses the
//! correlation maths from cross_market.rs instead of duplicating it. Lag is 0
//! because neither ticker is assumed to lead - unlike cross_market.rs (equities,
//! DEFAULT_LAG), this is an ad-hoc peer-to-peer pair chosen by the user.

use crate::{models, market_engine, analysis_engine};
use time::OffsetDateTime;
use super::cross_market::{
    align_and_correlate_lagged, daily_change_pct, dated_returns, overlapping_observations,
};
use super::error::CommandError;

const MAX_TICKER_LEN: usize = 15;

/// tickery Yahoo Finance: alfanumeryczne + ^ . = - (np. ^IXIC, GC=F, RDS-A.L)
pub(crate) fn validate_ticker(ticker: &str) -> Result<(), CommandError> {
    let trimmed = ticker.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_TICKER_LEN {
        return Err(CommandError::InvalidTicker(ticker.to_string()));
    }
    let valid = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '^' | '.' | '=' | '-'));
    if !valid {
        return Err(CommandError::InvalidTicker(ticker.to_string()));
    }
    Ok(())
}

pub(crate) async fn get_custom_pair_correlation_inner(
    ticker_a: String,
    ticker_b: String,
) -> Result<models::AnalyticalReport, CommandError> {
    validate_ticker(&ticker_a)?;
    validate_ticker(&ticker_b)?;
    let (ticker_a, ticker_b) = (ticker_a.trim(), ticker_b.trim());

    let (data_a, data_b) = tokio::join!(
        market_engine::fetch_market_data(ticker_a),
        market_engine::fetch_market_data(ticker_b),
    );
    let data_a = data_a.map_err(CommandError::MarketData)?;
    let data_b = data_b.map_err(CommandError::MarketData)?;

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
    Ok(build_custom_pair_report(ticker_a, &data_a, ticker_b, &data_b, &timestamp))
}

/// The report for one user-chosen pair; extracted so it can be tested without
/// network access.
fn build_custom_pair_report(
    ticker_a: &str,
    data_a: &[models::MarketData],
    ticker_b: &str,
    data_b: &[models::MarketData],
    timestamp: &str,
) -> models::AnalyticalReport {
    // Returns are taken from the candles, not from bare closes, so the session
    // dates survive as far as the join.
    let returns_a = dated_returns(data_a);
    let returns_b = dated_returns(data_b);
    let correlation = align_and_correlate_lagged(&returns_a, &returns_b, 0);
    let overlap = overlapping_observations(&returns_a, &returns_b);

    // Deliberate asymmetry: correlation describes the pair, but volatility, RSI,
    // MACD and price come from ticker A alone, since `AnalyticalReport` holds a
    // single indicator set. Ticker B feeds the correlation figure only, which the
    // UI must state explicitly so RSI is not read as an indicator of the pair.
    // The overlap count adds a third scope: it belongs to the pair, like the
    // correlation, and says nothing about the ticker A figures next to it.
    let closes_a: Vec<f64> = data_a.iter().map(|d| d.close).collect();
    let volatility = analysis_engine::calculate_volatility(data_a);
    let technicals = analysis_engine::calculate_technicals(data_a);
    let latest_close = closes_a.last().copied().unwrap_or(0.0);

    models::AnalyticalReport {
        symbol: format!("{}->{}", ticker_a, ticker_b),
        correlation,
        overlapping_observations: overlap,
        volatility,
        technicals,
        latest_close,
        daily_change_pct: daily_change_pct(&closes_a),
        timestamp: timestamp.to_string(),
    }
}

#[cfg(test)]
mod validate_ticker_tests {
    use super::*;

    #[test]
    fn accepts_typical_yahoo_tickers() {
        assert!(validate_ticker("AAPL").is_ok());
        assert!(validate_ticker("^IXIC").is_ok());
        assert!(validate_ticker("GC=F").is_ok());
        assert!(validate_ticker("RDS-A.L").is_ok());
    }

    #[test]
    fn rejects_empty_or_too_long() {
        assert!(validate_ticker("").is_err());
        assert!(validate_ticker("   ").is_err());
        assert!(validate_ticker(&"A".repeat(16)).is_err());
    }

    #[test]
    fn rejects_unexpected_characters() {
        assert!(validate_ticker("AAPL; DROP TABLE").is_err());
        assert!(validate_ticker("<script>").is_err());
        assert!(validate_ticker("has space").is_err());
    }
}

#[cfg(test)]
mod custom_pair_report_tests {
    use super::*;

    fn candle(day_offset: i64, close: f64) -> models::MarketData {
        let date = time::macros::date!(2026 - 01 - 05) + time::Duration::days(day_offset);
        models::MarketData {
            symbol: "TEST".to_string(),
            time: date.to_string(),
            timestamp: date.midnight().assume_utc().unix_timestamp(),
            open: close,
            high: close,
            low: close,
            close,
        }
    }

    fn series(days: impl IntoIterator<Item = i64>, base: f64) -> Vec<models::MarketData> {
        days.into_iter()
            .map(|d| candle(d, base + d as f64 + (d % 7) as f64 * 2.0))
            .collect()
    }

    #[test]
    fn partially_overlapping_calendars_measure_only_the_shared_sessions() {
        let data_a = series(0..60, 100.0);
        // Ticker B is closed every tenth session; six days of A have no counterpart.
        let data_b = series((0..60).filter(|d| d % 10 != 3), 40.0);

        let report = build_custom_pair_report("AAPL", &data_a, "GC=F", &data_b, "2026-01-01");

        assert!(report.correlation.is_some());
        assert_eq!(report.overlapping_observations, 53);

        // Price figures still come from ticker A alone.
        let closes_a: Vec<f64> = data_a.iter().map(|d| d.close).collect();
        assert_eq!(report.latest_close, *closes_a.last().unwrap());
        assert_eq!(report.daily_change_pct, daily_change_pct(&closes_a));
        assert_eq!(report.symbol, "AAPL->GC=F");
    }

    #[test]
    fn disjoint_calendars_leave_ticker_a_readings_intact() {
        let data_a = series(0..40, 100.0);
        let data_b = series(200..240, 40.0);

        let report = build_custom_pair_report("AAPL", &data_a, "GC=F", &data_b, "2026-01-01");

        assert_eq!(report.correlation, None);
        assert_eq!(report.overlapping_observations, 0);

        // Volatility, RSI, MACD and price depend on ticker A only; the failed join
        // must not drag them down with it.
        let closes_a: Vec<f64> = data_a.iter().map(|d| d.close).collect();
        let expected = analysis_engine::calculate_technicals(&data_a);
        assert_eq!(report.volatility, analysis_engine::calculate_volatility(&data_a));
        assert!(report.volatility > 0.0);
        assert_eq!(report.technicals.rsi, expected.rsi);
        assert_eq!(report.technicals.macd_line, expected.macd_line);
        assert_eq!(report.technicals.macd_signal, expected.macd_signal);
        assert_eq!(report.latest_close, *closes_a.last().unwrap());
        assert_eq!(report.daily_change_pct, daily_change_pct(&closes_a));
    }
}
