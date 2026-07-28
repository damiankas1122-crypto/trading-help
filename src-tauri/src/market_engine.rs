// src-tauri/src/market_engine.rs
use yahoo_finance_api as yf;
use crate::models::MarketData;
use time::{Duration, OffsetDateTime};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// Daily candles do not change during a session, yet a single user action
// (a tactic, say) can touch the same symbol several times. The cache cuts
// requests to the least reliable part of the stack.
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

type Cache = HashMap<String, (Instant, Vec<MarketData>)>;

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// The lock is never held across an await: it guards only map access, while
// the fetch itself happens outside it.
fn cached(symbol: &str) -> Option<Vec<MarketData>> {
    let guard = cache().lock().ok()?;
    let (fetched_at, data) = guard.get(symbol)?;
    (fetched_at.elapsed() < CACHE_TTL).then(|| data.clone())
}

fn store_in_cache(symbol: &str, data: &[MarketData]) {
    if let Ok(mut guard) = cache().lock() {
        guard.insert(symbol.to_string(), (Instant::now(), data.to_vec()));
    }
}

// Futures (GC=F, SI=F) occasionally return a candle containing NaN, and a
// single such close corrupts correlation, volatility, RSI and MACD for the
// whole series. Filtering here at the source spares every calculation from
// defending against it separately.
fn is_valid_candle(open: f64, high: f64, low: f64, close: f64) -> bool {
    open.is_finite() && high.is_finite() && low.is_finite() && close.is_finite()
}

pub async fn fetch_market_data(symbol: &str) -> Result<Vec<MarketData>, String> {
    if let Some(hit) = cached(symbol) {
        return Ok(hit);
    }

    let provider = yf::YahooConnector::new().map_err(|e| e.to_string())?;

    // 90 days (~63 sessions): MACD(12,26,9) needs ~34 points for a signal line.
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(90);

    let response = provider
        .get_quote_history(symbol, start, end)
        .await
        .map_err(|e| format!(
            "Nie udało się pobrać danych dla '{symbol}' z Yahoo Finance: {e}. \
             Sprawdź czy to prawidłowy ticker Yahoo (np. ^IXIC, GC=F), nie potoczna nazwa instrumentu."
        ))?;

    let quotes = response.quotes().map_err(|e| e.to_string())?;

    let history: Vec<MarketData> = quotes
        .iter()
        .filter(|q| is_valid_candle(q.open, q.high, q.low, q.close))
        .map(|q| {
            let date = OffsetDateTime::from_unix_timestamp(q.timestamp as i64)
                .map(|dt| dt.date().to_string())
                .unwrap_or_else(|_| q.timestamp.to_string());

            MarketData {
                symbol: symbol.to_string(),
                time: date,
                timestamp: q.timestamp as i64,
                open: q.open,
                high: q.high,
                low: q.low,
                close: q.close,
            }
        })
        .collect();

    store_in_cache(symbol, &history);
    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_candle_rejects_nan_in_any_field() {
        assert!(!is_valid_candle(f64::NAN, 1.0, 1.0, 1.0));
        assert!(!is_valid_candle(1.0, f64::NAN, 1.0, 1.0));
        assert!(!is_valid_candle(1.0, 1.0, f64::NAN, 1.0));
        assert!(!is_valid_candle(1.0, 1.0, 1.0, f64::NAN));
    }

    #[test]
    fn is_valid_candle_rejects_infinite_values() {
        assert!(!is_valid_candle(f64::INFINITY, 1.0, 1.0, 1.0));
        assert!(!is_valid_candle(1.0, 1.0, 1.0, f64::NEG_INFINITY));
    }

    #[test]
    fn is_valid_candle_accepts_normal_values() {
        assert!(is_valid_candle(100.0, 105.0, 99.0, 102.0));
    }
}