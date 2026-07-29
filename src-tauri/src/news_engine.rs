// src-tauri/src/news_engine.rs
use crate::models::NewsItem;
use std::time::Duration;

// The general "All News" feed is mostly world politics and almost never
// matches instrument keywords. Topical feeds hit far more often.
const NEWS_FEED_URLS: [&str; 2] = [
    "https://www.investing.com/rss/news_25.rss", // Stock Market News - NASDAQ/SP500
    "https://www.investing.com/rss/commodities_Metals.rss", // Metals Analysis - GOLD/SILVER
];

/// Fetches and merges the topical feeds; one dead feed does not sink the rest.
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

/// Filters news by instrument keywords, case-insensitively.
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

// A plain .contains() matched "gold" inside "goldman sachs", hence
// word-boundary matching.
//
// Boundaries are examined per character, never per byte: a multi-byte neighbour
// inspected as a single byte hits a continuation byte, which is not
// alphanumeric, so the boundary would be accepted even when a letter precedes
// the match ("gold" would match inside "żgold").
//
// Known limitation: the match is exact, so inflected forms are missed -
// "złoto" does not match "złota". Widening this to stemming would also start
// matching unrelated words, so keywords carry the burden instead.
fn contains_whole_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut search_from = 0;
    while let Some(pos) = haystack[search_from..].find(needle) {
        let start = search_from + pos;
        let end = start + needle.len();
        let before_is_boundary = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_is_boundary = haystack[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        if before_is_boundary && after_is_boundary {
            return true;
        }
        // Skip a whole character, not a byte: `start + 1` lands inside the first
        // character of the match whenever it is multi-byte, and slicing there
        // panics with "byte index is not a char boundary". `find` keeps its
        // optimised substring search, which a char_indices rewrite would lose.
        search_from = start + needle.chars().next().map_or(1, char::len_utf8);
    }
    false
}

/// Whether an instrument has a news source at all, kept separate from whether
/// that source returned anything. An empty keyword list used to mean both, so
/// the UI could only ever say "no news" - including for instruments nothing was
/// ever going to match. Mirrors `Option<&[NewsItem]>`, where `None` means the RSS
/// feed itself failed; three distinguishable states, three different sentences.
///
/// Today "has a source" is decided by the RSS feeds alone. `news_symbol`
/// (Finnhub, per listed ticker) and `keywords` (RSS, per thematic feed) are two
/// different channels that currently collapse into one state only because
/// Finnhub is not wired in yet. Adding it, or adding a macro feed, splits this
/// into two independent questions.
pub enum NewsSource {
    /// Never empty; the catalogue enforces it.
    Keywords(&'static [&'static str]),
    /// No feed covers this instrument (VIX, DXY, US10Y, OIL). The UI must say
    /// exactly that, never "no news".
    Unassigned,
}

/// Unknown ids also yield `Unassigned`: commands validate the instrument before
/// reaching this point, so a separate "unknown" variant would be dead code.
pub fn news_source_for(instrument: &str) -> NewsSource {
    match crate::catalog::find(instrument) {
        Some(entry) if !entry.keywords.is_empty() => NewsSource::Keywords(entry.keywords),
        _ => NewsSource::Unassigned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails loudly rather than filtering with an empty list, which would make
    /// every assertion below pass for the wrong reason.
    fn keywords(instrument: &str) -> &'static [&'static str] {
        match news_source_for(instrument) {
            NewsSource::Keywords(k) => k,
            NewsSource::Unassigned => panic!("{instrument} nie ma przypisanego źródła wiadomości"),
        }
    }

    fn item(title: &str) -> NewsItem {
        NewsItem {
            title: title.to_string(),
            description: String::new(),
            link: String::new(),
            published: String::new(),
        }
    }

    #[test]
    fn sourceless_instruments_report_unassigned_not_an_empty_keyword_list() {
        for instrument in ["VIX", "DXY", "US10Y", "OIL"] {
            assert!(
                matches!(news_source_for(instrument), NewsSource::Unassigned),
                "{instrument} powinien zgłaszać brak źródła"
            );
        }
    }

    #[test]
    fn instruments_with_a_source_report_a_non_empty_keyword_list() {
        for instrument in ["GOLD", "SILVER", "NASDAQ", "SP500", "GDX", "XLE"] {
            match news_source_for(instrument) {
                NewsSource::Keywords(k) => assert!(!k.is_empty()),
                NewsSource::Unassigned => panic!("{instrument} powinien mieć źródło"),
            }
        }
    }

    #[test]
    fn unknown_instrument_reports_unassigned() {
        assert!(matches!(news_source_for("BITCOIN"), NewsSource::Unassigned));
    }

    #[test]
    fn gold_keyword_does_not_match_goldman_sachs() {
        let news = vec![item("Goldman Sachs raises TOPIX 12-month target")];
        let result = filter_news_for_instrument(&news, keywords("GOLD"), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn gold_keyword_matches_standalone_word() {
        let news = vec![item("Gold rally risks becoming a bull trap")];
        let result = filter_news_for_instrument(&news, keywords("GOLD"), 5);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn multi_word_keyword_still_matches() {
        let news = vec![item("S&P 500 closes at record high")];
        let result = filter_news_for_instrument(&news, keywords("SP500"), 5);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn keyword_starting_with_a_multibyte_char_does_not_panic() {
        // The first occurrence is rejected (preceded by a letter), and only a
        // rejected match advances the cursor - by one byte previously, landing
        // inside 'ż' and panicking on the next slice. The standalone occurrence
        // after it is the real match.
        assert!(contains_whole_word("ażelazo żelazo dziś", "żelazo"));
        assert!(contains_whole_word("cena żelazo dziś", "żelazo"));
    }

    #[test]
    fn word_boundary_is_checked_on_characters_not_bytes() {
        // Byte-wise checks saw a continuation byte here, called it a boundary and
        // reported a match inside a longer word.
        assert!(!contains_whole_word("złotożółty pierścień", "złoto"));
        assert!(!contains_whole_word("żelazożelazo rośnie", "żelazo"));
        assert!(!contains_whole_word("żgold rally", "gold"));
        assert!(!contains_whole_word("złotoryja", "złoto"));
        // Not a substring at all, so it cannot match either way.
        assert!(!contains_whole_word("złotówka umacnia się", "złoto"));
    }

    #[test]
    fn inflected_forms_are_not_matched() {
        // Documented limitation of keyword matching, not an oversight.
        assert!(!contains_whole_word("cena złota spada", "złoto"));
    }

    #[test]
    fn characters_outside_the_bmp_around_the_match_are_handled() {
        assert!(contains_whole_word("📈 gold rally 🚀", "gold"));
        assert!(contains_whole_word("📈gold🚀", "gold"));
        assert!(!contains_whole_word("🚀goldman sachs", "gold"));
    }

    #[test]
    fn fed_keyword_does_not_match_federal_as_substring() {
        // "fed" and "federal reserve" are separate deliberate keywords: "fed"
        // must not match inside "federal", which the longer keyword covers.
        let news = vec![item("Federation of retailers reports steady sales")];
        let result = filter_news_for_instrument(&news, keywords("SP500"), 5);
        assert!(result.is_empty());
    }
}