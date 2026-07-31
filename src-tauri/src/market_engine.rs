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

/// Ascending order enforced at the source, like the NaN filter above. The crate
/// passes Yahoo's response order through verbatim (`quotes()` does not sort),
/// while `.last()`, `[len-2]` and `windows(2)` all over the codebase - up to
/// the `reference_price` persisted in tactics.json - assume newest-last. The
/// 2026-07-30 probe showed the order is currently ascending; one sort makes
/// that a property of this code instead of a property of Yahoo.
fn sorted_by_time(mut history: Vec<MarketData>) -> Vec<MarketData> {
    history.sort_by_key(|candle| candle.timestamp);
    history
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

/// Ceiling for one live-quote request. Lives out here because the crate's
/// builder silently drops a configured timeout: `YahooConnectorBuilder::build()`
/// (2.4.0) ignores its inner `ClientBuilder` and constructs a fresh client.
const LIVE_QUOTE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Ceiling for a 90-day history request - the same crate limitation as above,
/// so the same technique. Generous against the observed 1-3 s so a slow market
/// day never trips it; without any ceiling a hung connection froze market
/// context, briefing and tactic alike for as long as the OS kept the socket.
const HISTORY_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Shared connector for live quotes, built once. The 90-day history path keeps
/// its own per-call connector for now (CR-13 covers unifying that separately).
fn live_connector() -> Option<&'static yf::YahooConnector> {
    static CONNECTOR: OnceLock<Option<yf::YahooConnector>> = OnceLock::new();
    CONNECTOR.get_or_init(|| yf::YahooConnector::new().ok()).as_ref()
}

/// Pure assembly of a quote, so the validity rules are testable. `None` for a
/// non-positive or non-finite figure: rendering such a value would be
/// indistinguishable from a real reading (the A-01 rule, applied to quotes).
/// The previous close is held to the same bar because the daily change is
/// derived from it - a garbage denominator makes a confident-looking percent.
fn build_live_quote(
    instrument: &str,
    price: f64,
    previous_close: f64,
    market_time: i64,
) -> Option<crate::models::LiveQuote> {
    let valid = |value: f64| value.is_finite() && value > 0.0;
    if !valid(price) || !valid(previous_close) {
        return None;
    }
    Some(crate::models::LiveQuote {
        instrument: instrument.to_string(),
        price,
        previous_close,
        daily_change_pct: (price - previous_close) / previous_close * 100.0,
        market_time,
    })
}

/// Current quote from chart metadata (`regularMarketPrice`): a single-candle
/// range query, no history, and deliberately no cache - the candle cache exists
/// to protect the 90-day fetch, while this call is the one thing that must not
/// be five minutes old.
pub async fn fetch_live_quote(instrument: &str, symbol: &str) -> Result<crate::models::LiveQuote, MarketDataError> {
    let connector = live_connector().ok_or_else(|| MarketDataError::Fetch {
        symbol: symbol.to_string(),
        message: "nie udało się zainicjalizować klienta HTTP".to_string(),
    })?;

    let response = tokio::time::timeout(LIVE_QUOTE_TIMEOUT, connector.get_quote_range(symbol, "1d", "1d"))
        .await
        .map_err(|_| MarketDataError::Fetch {
            symbol: symbol.to_string(),
            message: format!("przekroczono czas oczekiwania ({} s)", LIVE_QUOTE_TIMEOUT.as_secs()),
        })?
        .map_err(|e| MarketDataError::Fetch {
            symbol: symbol.to_string(),
            message: e.to_string(),
        })?;

    let meta = response.metadata().map_err(|e| MarketDataError::Fetch {
        symbol: symbol.to_string(),
        message: e.to_string(),
    })?;

    build_live_quote(
        instrument,
        meta.regular_market_price,
        meta.chart_previous_close,
        meta.regular_market_time as i64,
    )
    .ok_or(MarketDataError::NoData { symbol: symbol.to_string() })
}

pub async fn fetch_market_data(symbol: &str) -> Result<Vec<MarketData>, MarketDataError> {
    let result = fetch_market_data_inner(symbol).await;
    // The log file is the only place a field failure remains visible after the
    // fact: the UI banner gets dismissed, and remote diagnosis of the
    // 2026-07-30 "stale prices" report failed precisely because market errors
    // never reached the log.
    if let Err(e) = &result {
        log::warn!("Market data fetch failed for {symbol}: {e}");
    }
    result
}

async fn fetch_market_data_inner(symbol: &str) -> Result<Vec<MarketData>, MarketDataError> {
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

    let response = tokio::time::timeout(
        HISTORY_FETCH_TIMEOUT,
        provider.get_quote_history(symbol, start, end),
    )
    .await
    .map_err(|_| MarketDataError::Fetch {
        symbol: symbol.to_string(),
        message: format!("przekroczono czas oczekiwania ({} s)", HISTORY_FETCH_TIMEOUT.as_secs()),
    })?
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
    let history = validate_history(symbol, sorted_by_time(history))?;
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

    fn candle_at(timestamp: i64, close: f64) -> MarketData {
        MarketData {
            symbol: "TEST".to_string(),
            time: timestamp.to_string(),
            timestamp,
            open: close,
            high: close,
            low: close,
            close,
        }
    }

    #[test]
    fn history_is_sorted_ascending_whatever_order_the_source_used() {
        let descending = vec![candle_at(300, 3.0), candle_at(200, 2.0), candle_at(100, 1.0)];
        let shuffled = vec![candle_at(200, 2.0), candle_at(300, 3.0), candle_at(100, 1.0)];

        for input in [descending, shuffled] {
            let sorted = sorted_by_time(input);
            let timestamps: Vec<i64> = sorted.iter().map(|c| c.timestamp).collect();
            assert_eq!(timestamps, vec![100, 200, 300]);
            // The newest close is what .last() serves to every consumer.
            assert_eq!(sorted.last().unwrap().close, 3.0);
        }
    }

    #[test]
    fn already_ascending_history_is_left_unchanged() {
        let ascending = vec![candle_at(100, 1.0), candle_at(200, 2.0), candle_at(300, 3.0)];
        let sorted = sorted_by_time(ascending.clone());
        let expected: Vec<i64> = ascending.iter().map(|c| c.timestamp).collect();
        assert_eq!(sorted.iter().map(|c| c.timestamp).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn live_quote_carries_price_change_and_time() {
        let quote = build_live_quote("GOLD", 4156.0, 4100.0, 1_785_400_000).expect("poprawne dane");
        assert_eq!(quote.instrument, "GOLD");
        assert_eq!(quote.price, 4156.0);
        assert_eq!(quote.market_time, 1_785_400_000);
        assert!((quote.daily_change_pct - (56.0 / 4100.0 * 100.0)).abs() < 1e-9);
    }

    #[test]
    fn live_quote_rejects_unusable_prices() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(build_live_quote("GOLD", bad, 4100.0, 0).is_none(), "cena {bad} przeszła");
            assert!(build_live_quote("GOLD", 4156.0, bad, 0).is_none(), "poprzednie zamknięcie {bad} przeszło");
        }
    }

    #[test]
    fn live_quote_change_sign_follows_direction() {
        assert!(build_live_quote("X", 110.0, 100.0, 0).unwrap().daily_change_pct > 0.0);
        assert!(build_live_quote("X", 90.0, 100.0, 0).unwrap().daily_change_pct < 0.0);
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
