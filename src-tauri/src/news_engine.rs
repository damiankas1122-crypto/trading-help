// src-tauri/src/news_engine.rs
use crate::models::NewsItem;
use std::time::Duration;

// ogólny feed "All News" to głównie światowa polityka - prawie nigdy nie
// łapie się na słowa kluczowe instrumentów. Tematyczne feedy trafiają dużo częściej.
const NEWS_FEED_URLS: [&str; 2] = [
    "https://www.investing.com/rss/news_25.rss", // Stock Market News - NASDAQ/SP500
    "https://www.investing.com/rss/commodities_Metals.rss", // Metals Analysis - GOLD/SILVER
];

/// pobiera i łączy tematyczne feedy; jeden padnięty feed nie wywala reszty
pub async fn fetch_all_news() -> Result<Vec<NewsItem>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (trading_help desktop app)")
        .build()
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    let mut last_error = None;

    for url in NEWS_FEED_URLS {
        match fetch_feed(&client, url).await {
            Ok(mut feed_items) => items.append(&mut feed_items),
            Err(e) => last_error = Some(e),
        }
    }

    if items.is_empty() {
        if let Some(e) = last_error {
            return Err(e);
        }
    }

    Ok(items)
}

async fn fetch_feed(client: &reqwest::Client, url: &str) -> Result<Vec<NewsItem>, String> {
    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Nie udało się pobrać newsów: {}", e))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let channel = rss::Channel::read_from(&bytes[..])
        .map_err(|e| format!("Błąd parsowania RSS: {}", e))?;

    Ok(channel
        .items()
        .iter()
        .map(|item| NewsItem {
            title: item.title().unwrap_or("").to_string(),
            description: item.description().unwrap_or("").to_string(),
            link: item.link().unwrap_or("").to_string(),
            published: item.pub_date().unwrap_or("").to_string(),
        })
        .collect())
}

/// Filtruje newsy po słowach kluczowych (case-insensitive) pasujących do instrumentu.
pub fn filter_news_for_instrument(news: &[NewsItem], keywords: &[&str], limit: usize) -> Vec<NewsItem> {
    news.iter()
        .filter(|item| {
            let haystack = format!("{} {}", item.title, item.description).to_lowercase();
            keywords.iter().any(|k| contains_whole_word(&haystack, &k.to_lowercase()))
        })
        .take(limit)
        .cloned()
        .collect()
}

// zwykłe .contains() łapało np. "gold" w środku "goldman sachs" - stąd
// dopasowanie tylko na granicy słowa
fn contains_whole_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut search_from = 0;
    while let Some(pos) = haystack[search_from..].find(needle) {
        let start = search_from + pos;
        let end = start + needle.len();
        let before_is_boundary = start == 0 || !haystack.as_bytes()[start - 1].is_ascii_alphanumeric();
        let after_is_boundary = end == haystack.len() || !haystack.as_bytes()[end].is_ascii_alphanumeric();
        if before_is_boundary && after_is_boundary {
            return true;
        }
        search_from = start + 1;
    }
    false
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(title: &str) -> NewsItem {
        NewsItem {
            title: title.to_string(),
            description: String::new(),
            link: String::new(),
            published: String::new(),
        }
    }

    #[test]
    fn gold_keyword_does_not_match_goldman_sachs() {
        let news = vec![item("Goldman Sachs raises TOPIX 12-month target")];
        let result = filter_news_for_instrument(&news, keywords_for("GOLD"), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn gold_keyword_matches_standalone_word() {
        let news = vec![item("Gold rally risks becoming a bull trap")];
        let result = filter_news_for_instrument(&news, keywords_for("GOLD"), 5);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn multi_word_keyword_still_matches() {
        let news = vec![item("S&P 500 closes at record high")];
        let result = filter_news_for_instrument(&news, keywords_for("SP500"), 5);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn fed_keyword_does_not_match_federal_as_substring() {
        // "fed" i "federal reserve" to osobne, celowe wpisy - "fed" nie powinno
        // łapać się w środku "federal" (to złapie osobny keyword "federal reserve")
        let news = vec![item("Federation of retailers reports steady sales")];
        let result = filter_news_for_instrument(&news, keywords_for("SP500"), 5);
        assert!(result.is_empty());
    }
}