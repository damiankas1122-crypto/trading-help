// src-tauri/src/market_engine.rs
use yahoo_finance_api as yf;
use crate::analysis_engine::MIN_CLOSES_FOR_INDICATORS;
use crate::models::MarketData;
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// Daily candles do not change during a session, yet a single user action
// (a tactic, say) can touch the same symbol several times. The cache cuts
// requests to the least reliable part of the stack.
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

// Soft cap on cached symbols. The catalogue holds 20, and the custom-pair
// builder adds two arbitrary tickers per query, so anything above ~40 is
// already headroom; 64 keeps the whole cache in the low hundreds of kilobytes
// (a symbol is ~60 candles) while never evicting anything a user is actively
// looking at. Without a cap the map only ever grew.
const MAX_CACHE_ENTRIES: usize = 64;

/// Market data failures, split by what the user can do about them.
#[derive(Error, Debug)]
pub enum MarketDataError {
    /// Transport or provider fault; retrying is meaningful.
    #[error("Nie udało się pobrać danych dla '{symbol}' z Yahoo Finance: {message}. \
             Sprawdź, czy to prawidłowy ticker Yahoo (np. ^IXIC, GC=F), nie potoczna nazwa instrumentu.")]
    Fetch { symbol: String, message: String },

    /// The provider answered but nothing usable came back - an outage on their
    /// side, not a property of the instrument. Kept apart from
    /// `InsufficientHistory` because only this one is worth retrying.
    #[error("Yahoo Finance nie zwróciło żadnych notowań dla '{symbol}'. \
             To awaria źródła danych, nie problem z instrumentem - spróbuj ponownie za chwilę.")]
    NoData { symbol: String },

    /// The instrument itself carries too short a history for the indicators.
    /// Retrying changes nothing, so the message must not suggest it.
    #[error("Za mało danych dla '{symbol}': {valid} poprawnych notowań, wymagane minimum {required}. \
             Ten instrument ma zbyt krótką historię, żeby policzyć wskaźniki - ponawianie nic nie zmieni.")]
    InsufficientHistory { symbol: String, valid: usize, required: usize },
}

type Cache = HashMap<String, (Instant, Vec<MarketData>)>;

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_fresh(fetched_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(fetched_at) < CACHE_TTL
}

fn lookup(cache: &Cache, symbol: &str, now: Instant) -> Option<Vec<MarketData>> {
    let (fetched_at, data) = cache.get(symbol)?;
    is_fresh(*fetched_at, now).then(|| data.clone())
}

/// Expired entries are dropped here rather than merely ignored on read: a stale
/// entry that is never requested again would otherwise sit in the map forever.
fn store(cache: &mut Cache, symbol: &str, data: &[MarketData], now: Instant) {
    cache.retain(|_, (fetched_at, _)| is_fresh(*fetched_at, now));

    if cache.len() >= MAX_CACHE_ENTRIES && !cache.contains_key(symbol) {
        let oldest = cache
            .iter()
            .min_by_key(|(_, (fetched_at, _))| *fetched_at)
            .map(|(key, _)| key.clone());
        if let Some(key) = oldest {
            cache.remove(&key);
        }
    }

    cache.insert(symbol.to_string(), (now, data.to_vec()));
}

// The lock is never held across an await: it guards only map access, while
// the fetch itself happens outside it.
fn cached(symbol: &str) -> Option<Vec<MarketData>> {
    let guard = cache().lock().ok()?;
    lookup(&guard, symbol, Instant::now())
}

fn store_in_cache(symbol: &str, data: &[MarketData]) {
    if let Ok(mut guard) = cache().lock() {
        store(&mut guard, symbol, data, Instant::now());
    }
}

// Futures (GC=F, SI=F) occasionally return a candle containing NaN, and a
// single such close corrupts correlation, volatility, RSI and MACD for the
// whole series. Filtering here at the source spares every calculation from
// defending against it separately.
fn is_valid_candle(open: f64, high: f64, low: f64, close: f64) -> bool {
    open.is_finite() && high.is_finite() && low.is_finite() && close.is_finite()
}

/// Runs after filtering, never before: the count that matters is how many
/// candles survived. An empty or too-short series returned as `Ok` reaches the
/// indicators, which answer with their defaults (price 0.0, RSI 50, MACD 0/0) -
/// values no caller can tell apart from a genuinely flat market, and which a
/// tactic then stores as if they were real readings.
fn validate_history(symbol: &str, history: Vec<MarketData>) -> Result<Vec<MarketData>, MarketDataError> {
    if history.is_empty() {
        return Err(MarketDataError::NoData { symbol: symbol.to_string() });
    }
    if history.len() < MIN_CLOSES_FOR_INDICATORS {
        return Err(MarketDataError::InsufficientHistory {
            symbol: symbol.to_string(),
            valid: history.len(),
            required: MIN_CLOSES_FOR_INDICATORS,
        });
    }
    Ok(history)
}

pub async fn fetch_market_data(symbol: &str) -> Result<Vec<MarketData>, MarketDataError> {
    if let Some(hit) = cached(symbol) {
        return Ok(hit);
    }

    let provider = yf::YahooConnector::new().map_err(|e| MarketDataError::Fetch {
        symbol: symbol.to_string(),
        message: e.to_string(),
    })?;

    // 90 days (~63 sessions) leaves room above MIN_CLOSES_FOR_INDICATORS even
    // for symbols that drop a fifth of their candles to nulls (DX-Y.NYB).
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(90);

    let response = provider
        .get_quote_history(symbol, start, end)
        .await
        .map_err(|e| MarketDataError::Fetch {
            symbol: symbol.to_string(),
            message: e.to_string(),
        })?;

    let quotes = response.quotes().map_err(|e| MarketDataError::Fetch {
        symbol: symbol.to_string(),
        message: e.to_string(),
    })?;

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

    // Validation precedes caching, so a bad response cannot be frozen in for the
    // whole TTL with no way to refresh it.
    let history = validate_history(symbol, history)?;
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

    fn series(len: usize) -> Vec<MarketData> {
        (0..len)
            .map(|i| MarketData {
                symbol: "TEST".to_string(),
                time: "2026-01-01".to_string(),
                timestamp: i as i64,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
            })
            .collect()
    }

    #[test]
    fn empty_series_is_an_error_not_an_empty_ok() {
        let err = validate_history("GC=F", Vec::new()).unwrap_err();
        assert!(matches!(err, MarketDataError::NoData { .. }));
        // The user is told the source failed, not that the instrument is thin.
        assert!(err.to_string().contains("awaria źródła"));
    }

    #[test]
    fn series_below_the_threshold_is_rejected_as_too_short() {
        let err = validate_history("GC=F", series(MIN_CLOSES_FOR_INDICATORS - 1)).unwrap_err();
        match err {
            MarketDataError::InsufficientHistory { valid, required, .. } => {
                assert_eq!(valid, MIN_CLOSES_FOR_INDICATORS - 1);
                assert_eq!(required, MIN_CLOSES_FOR_INDICATORS);
            }
            other => panic!("oczekiwano InsufficientHistory, otrzymano {other:?}"),
        }
        // Retrying cannot help here, so the message must not suggest it.
        assert!(!err.to_string().contains("spróbuj ponownie"));
    }

    #[test]
    fn series_exactly_at_the_threshold_passes() {
        let history = validate_history("GC=F", series(MIN_CLOSES_FOR_INDICATORS)).unwrap();
        assert_eq!(history.len(), MIN_CLOSES_FOR_INDICATORS);
    }

    #[test]
    fn threshold_matches_what_macd_actually_needs() {
        // A series one short of the threshold must not produce a MACD reading.
        let closes = vec![100.0; MIN_CLOSES_FOR_INDICATORS - 1];
        assert_eq!(crate::analysis_engine::calculate_macd(&closes), (0.0, 0.0));
    }

    #[test]
    fn fresh_entry_is_returned_and_expired_one_is_not() {
        let mut cache = Cache::new();
        let now = Instant::now();
        store(&mut cache, "GC=F", &series(40), now);

        assert!(lookup(&cache, "GC=F", now).is_some());
        assert!(lookup(&cache, "GC=F", now + CACHE_TTL).is_none());
    }

    #[test]
    fn expired_entries_are_evicted_on_write_not_just_ignored() {
        let mut cache = Cache::new();
        let now = Instant::now();
        store(&mut cache, "OLD", &series(40), now);

        store(&mut cache, "NEW", &series(40), now + CACHE_TTL);

        assert!(!cache.contains_key("OLD"), "wpis po TTL zostaje w mapie na zawsze");
        assert!(cache.contains_key("NEW"));
    }

    #[test]
    fn cache_size_is_capped_by_dropping_the_oldest_entry() {
        let mut cache = Cache::new();
        let now = Instant::now();
        // All entries stay fresh, so only the size cap can bound the map.
        for i in 0..MAX_CACHE_ENTRIES {
            store(&mut cache, &format!("SYM{i}"), &series(40), now + std::time::Duration::from_millis(i as u64));
        }
        assert_eq!(cache.len(), MAX_CACHE_ENTRIES);

        store(&mut cache, "OVERFLOW", &series(40), now + std::time::Duration::from_millis(MAX_CACHE_ENTRIES as u64));

        assert_eq!(cache.len(), MAX_CACHE_ENTRIES);
        assert!(!cache.contains_key("SYM0"), "najstarszy wpis powinien wypaść");
        assert!(cache.contains_key("OVERFLOW"));
    }

    #[test]
    fn refreshing_an_existing_symbol_does_not_evict_anything() {
        let mut cache = Cache::new();
        let now = Instant::now();
        for i in 0..MAX_CACHE_ENTRIES {
            store(&mut cache, &format!("SYM{i}"), &series(40), now);
        }

        store(&mut cache, "SYM0", &series(41), now);

        assert_eq!(cache.len(), MAX_CACHE_ENTRIES);
        assert_eq!(cache.get("SYM0").unwrap().1.len(), 41);
    }
}
