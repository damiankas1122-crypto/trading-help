// src-tauri/src/analysis_engine.rs
use crate::models::MarketData;

pub fn calculate_volatility(data: &[MarketData]) -> f64 {
    if data.len() < 2 { return 0.0; }
    let prices: Vec<f64> = data.iter().map(|d| d.close).collect();
    let mean: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
    let variance: f64 = prices.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;
    variance.sqrt()
}