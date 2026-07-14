// src-tauri/src/news_engine.rs
use crate::models::NewsItem;
use std::time::Duration;

const NEWS_FEED_URL: &str = "https://www.investing.com/rss/news.rss";

/// Pobiera i parsuje ogólny feed RSS z Investing.com.
pub async fn fetch_all_news() -> Result<Vec<NewsItem>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (trading_help desktop app)")
        .build()
        .map_err(|e| e.to_string())?;

    let bytes = client
        .get(NEWS_FEED_URL)
        .send()
        .await
        .map_err(|e| format!("Nie udało się pobrać newsów: {}", e))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let channel = rss::Channel::read_from(&bytes[..])
        .map_err(|e| format!("Błąd parsowania RSS: {}", e))?;

    let items = channel
        .items()
        .iter()
        .map(|item| NewsItem {
            title: item.title().unwrap_or("").to_string(),
            description: item.description().unwrap_or("").to_string(),
            link: item.link().unwrap_or("").to_string(),
            published: item.pub_date().unwrap_or("").to_string(),
        })
        .collect();

    Ok(items)
}

/// Filtruje newsy po słowach kluczowych (case-insensitive) pasujących do instrumentu.
pub fn filter_news_for_instrument(news: &[NewsItem], keywords: &[&str], limit: usize) -> Vec<NewsItem> {
    news.iter()
        .filter(|item| {
            let haystack = format!("{} {}", item.title, item.description).to_lowercase();
            keywords.iter().any(|k| haystack.contains(&k.to_lowercase()))
        })
        .take(limit)
        .cloned()
        .collect()
}

pub fn keywords_for(instrument: &str) -> &'static [&'static str] {
    match instrument {
        "NASDAQ" => &["nasdaq", "tech stocks", "technology sector", "big tech", "ai stocks"],
        "SP500" => &["s&p 500", "s&p500", "wall street", "stock market", "equities", "fed", "federal reserve", "inflation"],
        "GOLD" => &["gold", "bullion", "safe haven", "precious metal"],
        "SILVER" => &["silver"],
        _ => &[],
    }
}