// src-tauri/src/market_engine.rs
use yahoo_finance_api as yf;
use crate::models::MarketData;
use time::{Duration, OffsetDateTime};

pub async fn fetch_market_data(symbol: &str) -> Result<Vec<MarketData>, String> {
    let provider = yf::YahooConnector::new().map_err(|e| e.to_string())?;

    // Pobieramy dane z ostatnich 30 dni
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(30);

    let response = provider
        .get_quote_history(symbol, start, end)
        .await
        .map_err(|e| format!("Błąd API Yahoo: {}", e))?;

    let quotes = response.quotes().map_err(|e| e.to_string())?;

    let history: Vec<MarketData> = quotes
        .iter()
        .map(|q| {
            let date = OffsetDateTime::from_unix_timestamp(q.timestamp as i64)
                .map(|dt| dt.date().to_string())
                .unwrap_or_else(|_| q.timestamp.to_string());

            MarketData {
                symbol: symbol.to_string(),
                time: date,
                open: q.open,
                high: q.high,
                low: q.low,
                close: q.close,
            }
        })
        .collect();

    Ok(history)
}