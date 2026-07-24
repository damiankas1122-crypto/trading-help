// src-tauri/src/analysis_engine.rs
use crate::models::MarketData;

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
#[cfg(test)]
mod tests {
    use super::*;

    fn candle(close: f64) -> MarketData {
        MarketData {
            symbol: "TEST".to_string(),
            time: "2026-01-01".to_string(),
            open: close,
            high: close,
            low: close,
            close,
        }
    }

    #[test]
    fn volatility_is_computed_from_percentage_returns_not_absolute_prices() {
        // Regresja: kiedyś liczono zmienność z cen bezwzględnych, przez co
        // drogi instrument (np. 20000 pkt) sztucznie wyglądał na bardziej
        // zmienny niż tani (np. 100 pkt) przy identycznym % ruchu dziennym.
        let cheap = vec![candle(100.0), candle(101.0), candle(99.0), candle(100.0)];
        let expensive = vec![candle(20000.0), candle(20200.0), candle(19800.0), candle(20000.0)];

        let vol_cheap = calculate_volatility(&cheap);
        let vol_expensive = calculate_volatility(&expensive);

        // Identyczny % ruch dzień do dnia -> identyczna zmienność w %,
        // niezależnie od poziomu ceny instrumentu.
        assert!((vol_cheap - vol_expensive).abs() < 1e-9);
    }

    #[test]
    fn volatility_is_zero_for_fewer_than_two_data_points() {
        assert_eq!(calculate_volatility(&[]), 0.0);
        assert_eq!(calculate_volatility(&[candle(100.0)]), 0.0);
    }

    #[test]
    fn volatility_ignores_zero_price_to_avoid_division_by_zero() {
        // Regresja: dzielenie przez zero gdy poprzednia świeca miała close=0.0
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
}