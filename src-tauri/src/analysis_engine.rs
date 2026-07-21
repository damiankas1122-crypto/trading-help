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