//! Gemini integration: provider selection (`AiProvider`), typed errors
//! (`AiEngineError`) and the public module API re-exported from the private
//! submodules below. Submodules see each other only through what this file
//! re-exports explicitly or through the `pub(crate)` helpers defined here
//! (`label_to_tv_ticker`, `format_news_lines`, `strip_json_fence`).

use crate::models::NewsItem;
use thiserror::Error;

mod gemini_client;
mod correlation_pine;
mod tactic;
mod briefing;

pub use gemini_client::GeminiProvider;
pub use correlation_pine::{
    find_strongest_equity_pair, generate_correlation_pine_script, explain_correlation_script,
    generate_gsr_pine_script, explain_gsr_script,
};
pub use tactic::generate_trading_tactic;
pub use briefing::generate_instrument_briefing;

/// Typed Gemini errors, replacing `Result<T, String>` and text matching.
#[derive(Error, Debug)]
pub enum AiEngineError {
    #[error("Brak klucza API Gemini. Ustaw go w ustawieniach aplikacji (pierwsze uruchomienie lub panel ustawień).")]
    MissingApiKey,

    #[error("Nie udało się zainicjalizować klienta HTTP: {0}")]
    ClientBuildFailed(String),

    #[error("Błąd połączenia z Gemini API: {0}")]
    ConnectionFailed(String),

    #[error("Błąd parsowania odpowiedzi Gemini: {0}")]
    ResponseParseFailed(String),

    #[error("Gemini nie zwróciło żadnej treści")]
    EmptyResponse,

    #[error("Przekroczono darmowy limit zapytań Gemini API (5 zapytań/minutę). Spróbuj ponownie za chwilę.")]
    RateLimitExceeded,

    /// 4xx - the request itself is at fault (bad key, malformed body). Retrying
    /// cannot help, so the message must not suggest it. The raw body goes to
    /// stderr, never to the user.
    #[error("{message}")]
    ApiClientError { status: u16, message: String },

    /// 5xx - a fault on Google's side, where retrying is meaningful.
    #[error("Gemini API jest chwilowo niedostępne (błąd {status}, nieudane próby: {attempts}). Spróbuj ponownie za chwilę.")]
    ApiServerError { status: u16, attempts: u32 },
}

/// Message for 4xx: factual, and never implies that retrying would help.
pub(crate) fn client_error_message(status: u16) -> String {
    match status {
        401 | 403 => "Klucz API Gemini został odrzucony. Sprawdź, czy jest poprawny i aktywny \
                      - możesz go zmienić w Ustawieniach."
            .to_string(),
        400 => "Gemini odrzuciło treść zapytania (400). To błąd aplikacji, nie Twoich danych \
                - zgłoś go, jeśli się powtarza."
            .to_string(),
        404 => "Nie znaleziono modelu Gemini (404). Model mógł zostać wycofany - to błąd \
                aplikacji, zgłoś go."
            .to_string(),
        other => format!(
            "Gemini API odrzuciło zapytanie (błąd {other}). Szczegóły techniczne zapisano w logu."
        ),
    }
}

/// Groundwork for multi-provider support: a new provider is a new impl of this
/// trait and nothing else changes.
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn generate(&self, prompt: String) -> Result<String, AiEngineError>;
}

/// Maps an instrument label to a TradingView ticker. Shared by
/// correlation_pine.rs (equity/GSR pair) and briefing.rs (per-instrument signal).
pub(crate) fn label_to_tv_ticker(label: &str) -> &'static str {
    match label {
        "NASDAQ" => "NASDAQ:IXIC",
        "SP500" => "SP:SPX",
        "GOLD" => "TVC:GOLD",
        "SILVER" => "TVC:SILVER",
        _ => "SP:SPX",
    }
}

/// Gemini sometimes wraps the response in a ```json fence; strip it before parsing.
pub(crate) fn strip_json_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.strip_suffix("```").unwrap_or(s))
        .unwrap_or(trimmed)
        .trim()
}

/// Shared by briefing.rs and tactic.rs, which build prompts from the same news
/// list. `None` means the RSS feed is unavailable, which differs from "the feed
/// works but nothing matches". Without that distinction the model claimed there
/// was no news even when the feed had simply failed.
pub(crate) fn format_news_lines(news: Option<&[NewsItem]>) -> String {
    match news {
        None => "(źródło newsów chwilowo niedostępne - NIE twierdź, że brak jest \
                 nowych wiadomości; po prostu pomiń wątek newsowy i oprzyj się na \
                 danych liczbowych)"
            .to_string(),
        Some([]) => "(feed działa, ale żaden news nie pasuje do tego instrumentu)".to_string(),
        Some(items) => items
            .iter()
            .map(|n| format!("- {}: {}", n.title, n.description))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[cfg(test)]
mod error_message_tests {
    use super::*;

    #[test]
    fn bad_key_message_points_at_settings_not_at_retrying() {
        for status in [401, 403] {
            let msg = client_error_message(status);
            assert!(msg.contains("Ustawieniach"), "status {status}: {msg}");
            // Regression: a 4xx message must not promise that retrying helps.
            assert!(!msg.contains("Spróbuj ponownie"), "status {status}: {msg}");
        }
    }

    #[test]
    fn client_error_messages_never_suggest_retry() {
        for status in [400, 404, 418] {
            let msg = client_error_message(status);
            assert!(!msg.contains("Spróbuj ponownie"), "status {status}: {msg}");
        }
    }

    #[test]
    fn server_error_keeps_retry_framing_and_hides_raw_body() {
        let err = AiEngineError::ApiServerError { status: 503, attempts: 3 };
        let msg = err.to_string();
        assert!(msg.contains("Spróbuj ponownie"));
        assert!(msg.contains("503"));
    }

    #[test]
    fn client_error_message_is_the_whole_user_facing_text() {
        // #[error("{message}")] guarantees no raw Google JSON can leak in here.
        let err = AiEngineError::ApiClientError {
            status: 403,
            message: client_error_message(403),
        };
        assert_eq!(err.to_string(), client_error_message(403));
    }

    #[test]
    fn news_lines_distinguish_missing_feed_from_no_matches() {
        let unavailable = format_news_lines(None);
        let no_matches = format_news_lines(Some(&[]));

        assert_ne!(unavailable, no_matches);
        // Regression: on a dead feed the model must not claim "no news".
        assert!(unavailable.contains("NIE twierdź"));
    }
}
