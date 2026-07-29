//! Gemini integration: provider selection (`AiProvider`), typed errors
//! (`AiEngineError`) and the public module API re-exported from the private
//! submodules below. Submodules see each other only through what this file
//! re-exports explicitly or through the `pub(crate)` helpers defined here
//! (`label_to_tv_ticker`, `format_news_lines`, `strip_json_fence`).

use crate::models::NewsItem;
use thiserror::Error;

pub mod cancel;
mod gemini_client;
mod correlation_pine;
mod tactic;
mod briefing;

pub use gemini_client::{CallContext, GeminiProvider, GEMINI_MODEL};
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

    /// Raised before the Gemini call, so an instrument missing from the
    /// catalogue costs no quota.
    #[error("Instrument {0} nie jest obsługiwany przez aplikację.")]
    UnknownInstrument(String),

    #[error("Nie udało się zainicjalizować klienta HTTP: {0}")]
    ClientBuildFailed(String),

    #[error("Błąd połączenia z Gemini API: {0}")]
    ConnectionFailed(String),

    /// Not a failure: the user asked for it. The frontend recognises this
    /// variant and returns to idle instead of showing an error panel.
    #[error("Analiza została przerwana.")]
    Cancelled,

    /// Hard ceiling covering every attempt and backoff together, so a single
    /// click cannot hang for minutes.
    #[error("Przekroczono limit czasu ({seconds} s) na odpowiedź od Gemini. Sprawdź połączenie i spróbuj ponownie.")]
    TimeBudgetExceeded { seconds: u64 },

    #[error("Błąd parsowania odpowiedzi Gemini: {0}")]
    ResponseParseFailed(String),

    #[error("Gemini nie zwróciło żadnej treści")]
    EmptyResponse,

    /// Per-minute cap: waiting genuinely helps.
    #[error("Przekroczono minutowy limit zapytań Gemini API. Spróbuj ponownie za chwilę.")]
    RateLimitExceeded,

    /// Per-day cap on the free tier. Telling the user to "try again in a moment"
    /// here is simply false - the next attempt that can succeed is tomorrow, and
    /// every attempt before then still consumes quota.
    #[error("Wyczerpano dzienny limit zapytań do Gemini (darmowy tier: {limit} zapytań na dobę dla tego modelu). \
             Limit odnawia się o północy czasu pacyficznego, czyli około 9:00 czasu polskiego.")]
    DailyQuotaExhausted { limit: u32 },

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

/// Maps an instrument id to a TradingView ticker via the catalogue. Returns
/// `None` for unknown ids: an unrecognised label used to render as "SP:SPX",
/// which handed the user a working Pine Script pointing at S&P 500 instead of
/// the instrument they asked about.
pub(crate) fn label_to_tv_ticker(label: &str) -> Option<&'static str> {
    crate::catalog::find(label).map(|entry| entry.tv_ticker)
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
