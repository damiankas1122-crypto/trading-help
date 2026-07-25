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
    find_strongest_pair, generate_correlation_pine_script, explain_correlation_script,
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

    #[error("Gemini API zwróciło błąd {status}: {body} (model chwilowo przeciążony po {attempts} próbach - spróbuj ponownie za chwilę)")]
    ApiError {
        status: u16,
        body: String,
        attempts: u32,
    },
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
/// samą listą newsów
pub(crate) fn format_news_lines(news: &[NewsItem]) -> String {
    if news.is_empty() {
        return "(brak dopasowanych newsów w feedzie)".to_string();
    }
    news.iter()
        .map(|n| format!("- {}: {}", n.title, n.description))
        .collect::<Vec<_>>()
        .join("\n")
}
