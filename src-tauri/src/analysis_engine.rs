// src-tauri/src/analysis_engine.rs
use crate::models::{MarketData, TechnicalIndicators};

pub fn calculate_volatility(data: &[MarketData]) -> f64 {
    if data.len() < 2 { return 0.0; }
    let closes: Vec<f64> = data.iter().map(|d| d.close).collect();
    let returns: Vec<f64> = closes
        .windows(2)
        .filter(|w| w[0] != 0.0)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect();
    if returns.len() < 2 { return 0.0; }
    let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    variance.sqrt() * 100.0 // odchylenie standardowe dziennych zwrotów w %
}

const RSI_PERIOD: usize = 14;
const MACD_FAST: usize = 12;
const MACD_SLOW: usize = 26;
const MACD_SIGNAL: usize = 9;

/// Shortest close series that yields a real MACD signal line: the slow EMA
/// consumes `MACD_SLOW` points before the first MACD value, and the signal line
/// needs `MACD_SIGNAL` MACD values on top. Exported so data fetching can reject
/// a series up front instead of letting the indicators return their
/// "not computed" defaults, which are indistinguishable from a neutral market.
pub const MIN_CLOSES_FOR_INDICATORS: usize = MACD_SLOW + MACD_SIGNAL - 1;

/// Wilder's RSI(14). Falls back to 50.0 on insufficient data - a default, not a signal.
pub fn calculate_rsi(closes: &[f64]) -> f64 {
    if closes.len() < RSI_PERIOD + 1 {
        return 50.0;
    }

    let changes: Vec<f64> = closes.windows(2).map(|w| w[1] - w[0]).collect();

    let mut avg_gain = changes[..RSI_PERIOD].iter().filter(|c| **c > 0.0).sum::<f64>() / RSI_PERIOD as f64;
    let mut avg_loss = changes[..RSI_PERIOD].iter().filter(|c| **c < 0.0).map(|c| -c).sum::<f64>() / RSI_PERIOD as f64;

    for &change in &changes[RSI_PERIOD..] {
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };
        avg_gain = (avg_gain * (RSI_PERIOD as f64 - 1.0) + gain) / RSI_PERIOD as f64;
        avg_loss = (avg_loss * (RSI_PERIOD as f64 - 1.0) + loss) / RSI_PERIOD as f64;
    }

    if avg_loss == 0.0 {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

/// EMA seeded with the SMA of the first `period` values, matching TradingView.
/// The last element is the EMA at the most recent data point.
fn ema_series(values: &[f64], period: usize) -> Vec<f64> {
    if values.len() < period {
        return Vec::new();
    }
    let k = 2.0 / (period as f64 + 1.0);
    let seed = values[..period].iter().sum::<f64>() / period as f64;
    let mut result = Vec::with_capacity(values.len() - period + 1);
    result.push(seed);
    for &v in &values[period..] {
        let prev = *result.last().unwrap();
        result.push(v * k + prev * (1.0 - k));
    }
    result
}

/// MACD(12,26,9). Returns (0.0, 0.0) when history is too short - meaning
/// "not computed", not a neutral signal.
pub fn calculate_macd(closes: &[f64]) -> (f64, f64) {
    if closes.len() < MIN_CLOSES_FOR_INDICATORS {
        return (0.0, 0.0);
    }

    let ema_fast = ema_series(closes, MACD_FAST);
    let ema_slow = ema_series(closes, MACD_SLOW);
    let offset = MACD_SLOW - MACD_FAST;

    let macd_series: Vec<f64> = ema_slow
        .iter()
        .enumerate()
        .map(|(i, slow)| ema_fast[i + offset] - slow)
        .collect();

    let signal_series = ema_series(&macd_series, MACD_SIGNAL);

    let macd_line = *macd_series.last().unwrap();
    let signal_line = *signal_series.last().unwrap_or(&0.0);
    (macd_line, signal_line)
}

pub fn calculate_technicals(data: &[MarketData]) -> TechnicalIndicators {
    let closes: Vec<f64> = data.iter().map(|d| d.close).collect();
    let rsi = calculate_rsi(&closes);
    let (macd_line, macd_signal) = calculate_macd(&closes);
    TechnicalIndicators { rsi, macd_line, macd_signal }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn candle(close: f64) -> MarketData {
        MarketData {
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
    fn volatility_is_computed_from_percentage_returns_not_absolute_prices() {
        // Regression: volatility was once computed from absolute prices, which
        // made an expensive instrument look more volatile than a cheap one at
        // identical daily percentage moves.
        let cheap = vec![candle(100.0), candle(101.0), candle(99.0), candle(100.0)];
        let expensive = vec![candle(20000.0), candle(20200.0), candle(19800.0), candle(20000.0)];

        let vol_cheap = calculate_volatility(&cheap);
        let vol_expensive = calculate_volatility(&expensive);

        // Identical daily percentage moves must yield identical volatility,
        // regardless of price level.
        assert!((vol_cheap - vol_expensive).abs() < 1e-9);
    }

    #[test]
    fn volatility_is_zero_for_fewer_than_two_data_points() {
        assert_eq!(calculate_volatility(&[]), 0.0);
        assert_eq!(calculate_volatility(&[candle(100.0)]), 0.0);
    }

    #[test]
    fn volatility_ignores_zero_price_to_avoid_division_by_zero() {
        // Regression: division by zero when the previous candle had close=0.0.
        let data = vec![candle(0.0), candle(100.0), candle(101.0)];
        let result = calculate_volatility(&data);
        assert!(result.is_finite());
    }

    #[test]
    fn volatility_is_higher_for_more_volatile_series() {
        let stable = vec![candle(100.0), candle(100.5), candle(99.5), candle(100.0)];
        let volatile = vec![candle(100.0), candle(110.0), candle(90.0), candle(105.0)];

        assert!(calculate_volatility(&volatile) > calculate_volatility(&stable));
    }

    #[test]
    fn rsi_returns_neutral_default_when_not_enough_data() {
        let closes: Vec<f64> = (0..10).map(|i| 100.0 + i as f64).collect();
        assert_eq!(calculate_rsi(&closes), 50.0);
    }

    #[test]
    fn rsi_is_100_when_every_change_is_a_gain() {
        // Gains only means zero losses, so RSI saturates at 100 (avg_loss == 0.0).
        let closes: Vec<f64> = (0..16).map(|i| 100.0 + i as f64).collect();
        assert_eq!(calculate_rsi(&closes), 100.0);
    }

    #[test]
    fn rsi_is_higher_for_uptrend_than_downtrend() {
        let uptrend: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 0.5).collect();
        let downtrend: Vec<f64> = (0..20).map(|i| 100.0 - i as f64 * 0.5).collect();

        assert!(calculate_rsi(&uptrend) > calculate_rsi(&downtrend));
    }

    #[test]
    fn macd_returns_zero_when_not_enough_data() {
        let closes: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        assert_eq!(calculate_macd(&closes), (0.0, 0.0));
    }

    #[test]
    fn macd_line_is_positive_for_sustained_uptrend() {
        // A sustained rise keeps EMA(12) above EMA(26), so MACD stays positive.
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + i as f64).collect();
        let (macd_line, _signal) = calculate_macd(&closes);
        assert!(macd_line > 0.0);
    }

    #[test]
    fn macd_line_is_negative_for_sustained_downtrend() {
        let closes: Vec<f64> = (0..40).map(|i| 200.0 - i as f64).collect();
        let (macd_line, _signal) = calculate_macd(&closes);
        assert!(macd_line < 0.0);
    }
}