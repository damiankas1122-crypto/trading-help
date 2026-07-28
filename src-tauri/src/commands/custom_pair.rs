//! Arbitrary correlation pair (X vs Y), independent of the watchlist. Reuses the
//! correlation maths from cross_market.rs instead of duplicating it. Lag is 0
//! because neither ticker is assumed to lead - unlike cross_market.rs (equities,
//! DEFAULT_LAG), this is an ad-hoc peer-to-peer pair chosen by the user.

use crate::{models, market_engine, analysis_engine};
use time::OffsetDateTime;
use super::cross_market::{to_returns, align_and_correlate_lagged, daily_change_pct};
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

    let closes_a: Vec<f64> = data_a.iter().map(|d| d.close).collect();
    let closes_b: Vec<f64> = data_b.iter().map(|d| d.close).collect();

    let returns_a = to_returns(&closes_a);
    let returns_b = to_returns(&closes_b);
    let correlation = align_and_correlate_lagged(&returns_a, &returns_b, 0);

    // Deliberate asymmetry: correlation describes the pair, but volatility, RSI,
    // MACD and price come from ticker A alone, since `AnalyticalReport` holds a
    // single indicator set. Ticker B feeds the correlation figure only, which the
    // UI must state explicitly so RSI is not read as an indicator of the pair.
    let volatility = analysis_engine::calculate_volatility(&data_a);
    let technicals = analysis_engine::calculate_technicals(&data_a);
    let latest_close = closes_a.last().copied().unwrap_or(0.0);
    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();

    Ok(models::AnalyticalReport {
        symbol: format!("{}->{}", ticker_a, ticker_b),
        correlation,
        volatility,
        technicals,
        latest_close,
        daily_change_pct: daily_change_pct(&closes_a),
        timestamp,
    })
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
