//! Integracja z Gemini: wybór dostawcy (`AiProvider`), typowane błędy
//! (`AiEngineError`) i publiczne API modułu - reeksport z prywatnych
//! submodułów niżej. Submoduły widzą się nawzajem tylko przez to, co ten
//! plik jawnie reeksportuje albo przez `pub(crate)` helpery zdefiniowane
//! tutaj (`label_to_tv_ticker`, `format_news_lines`, `strip_json_fence`).

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

/// błędy Gemini - zamiast Result<T, String> i zgadywania po tekście
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

    /// 4xx - problem po NASZEJ stronie (zły klucz, złe żądanie). Ponawianie nic
    /// nie da, więc komunikat nie zachęca do "spróbuj ponownie". Surowe body
    /// leci na stderr, nie do usera.
    #[error("{message}")]
    ApiClientError { status: u16, message: String },

    /// 5xx - problem po stronie Google. Tu ponawianie ma sens.
    #[error("Gemini API jest chwilowo niedostępne (błąd {status}, nieudane próby: {attempts}). Spróbuj ponownie za chwilę.")]
    ApiServerError { status: u16, attempts: u32 },
}

/// komunikat dla 4xx - rzeczowy, bez sugerowania że retry pomoże
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

/// pod multi-provider (Etap 6) - nowy dostawca to nowa impl tego trait,
/// zero zmian gdzie indziej
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn generate(&self, prompt: String) -> Result<String, AiEngineError>;
}

/// mapowanie etykiety instrumentu na ticker TradingView - współdzielone
/// między correlation_pine.rs (para equity/GSR) i briefing.rs (sygnał per instrument)
pub(crate) fn label_to_tv_ticker(label: &str) -> &'static str {
    match label {
        "NASDAQ" => "NASDAQ:IXIC",
        "SP500" => "SP:SPX",
        "GOLD" => "TVC:GOLD",
        "SILVER" => "TVC:SILVER",
        _ => "SP:SPX",
    }
}

/// Gemini czasem i tak owija odpowiedź w ```json - zdejmujemy to przed parsowaniem
pub(crate) fn strip_json_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.strip_suffix("```").unwrap_or(s))
        .unwrap_or(trimmed)
        .trim()
}

/// współdzielone między briefing.rs i tactic.rs - obie generują prompt z tą
/// samą listą newsów. `None` = feed RSS niedostępny (co jest czymś innym niż
/// "feed działa, ale nic nie pasuje do instrumentu") - bez tego rozróżnienia
/// AI twierdziło "brak nowych wiadomości" także gdy RSS po prostu padł
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
            // regresja: 4xx nie może obiecywać, że ponowienie pomoże
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
        // #[error("{message}")] - żaden surowy JSON od Google nie może się tu wkleić
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
        // regresja B-08: przy padniętym feedzie AI nie może twierdzić "brak wiadomości"
        assert!(unavailable.contains("NIE twierdź"));
    }
}
