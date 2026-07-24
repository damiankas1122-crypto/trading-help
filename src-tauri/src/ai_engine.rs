// src-tauri/src/ai_engine.rs
use crate::models::{AnalyticalReport, InstrumentBriefing, NewsItem};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const GEMINI_MODEL: &str = "gemini-3.5-flash";

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiErrorDetail {
    #[serde(rename = "retryDelay")]
    retry_delay: Option<String>,
}

#[derive(Deserialize)]
struct GeminiErrorBody {
    status: Option<String>,
    details: Option<Vec<GeminiErrorDetail>>,
}

#[derive(Deserialize)]
struct GeminiErrorWrapper {
    error: GeminiErrorBody,
}

/// Typowane błędy silnika AI (Gemini). Zastępuje wcześniejsze Result<T, String>,
/// gdzie caller musiał zgadywać rodzaj błędu przez parsowanie treści stringa.
/// Każdy wariant ma czytelny komunikat PL przez #[error(...)] - .to_string()
/// generuje dokładnie ten sam tekst co dawniej trafiał do usera, więc
/// zachowanie widoczne dla frontendu się nie zmienia, tylko wewnętrzna
/// obsługa staje się bezpieczniejsza (dopasowanie po wariancie, nie po tekście).
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
/// Abstrakcja nad dostawcą AI. Dziś jedyna implementacja to GeminiProvider,
/// ale to jest właśnie mechanizm potrzebny pod Etap 6 (multi-provider:
/// Gemini/OpenAI/Claude) - dodanie nowego dostawcy w przyszłości to jedna
/// nowa implementacja tego trait, zero zmian w generate_instrument_briefing.
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn generate(&self, prompt: String) -> Result<String, AiEngineError>;
}

pub struct GeminiProvider;

#[async_trait::async_trait]
impl AiProvider for GeminiProvider {
    async fn generate(&self, prompt: String) -> Result<String, AiEngineError> {
        call_gemini(prompt).await
    }
}

async fn call_gemini(prompt: String) -> Result<String, AiEngineError> {
    let api_key = crate::keychain::get_gemini_api_key().map_err(|_| AiEngineError::MissingApiKey)?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        GEMINI_MODEL
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| AiEngineError::ClientBuildFailed(e.to_string()))?;

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: prompt }],
        }],
    };

    const MAX_RETRIES: u32 = 3;
    let mut last_error: Option<AiEngineError> = None;

    for attempt in 0..MAX_RETRIES {
        let res = client
            .post(&url)
            .header("x-goog-api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiEngineError::ConnectionFailed(e.to_string()))?;

        let status = res.status();

        if status.is_success() {
            let parsed: GeminiResponse = res
                .json()
                .await
                .map_err(|e| AiEngineError::ResponseParseFailed(e.to_string()))?;

            return parsed
                .candidates
                .first()
                .and_then(|c| c.content.parts.first())
                .map(|p| p.text.trim().to_string())
                .ok_or(AiEngineError::EmptyResponse);
        }

        let is_retryable = status.as_u16() == 503 || status.as_u16() == 429;
        let text = res.text().await.unwrap_or_default();

        // Rozpoznaj konkretnie RESOURCE_EXHAUSTED (limit darmowego tieru) i odczytaj
        // sugerowany retryDelay od Google zamiast pokazywać userowi surowy JSON.
        let parsed_error: Option<GeminiErrorWrapper> = serde_json::from_str(&text).ok();
        let is_resource_exhausted = parsed_error
            .as_ref()
            .and_then(|w| w.error.status.as_deref())
            .map(|s| s == "RESOURCE_EXHAUSTED")
            .unwrap_or(false);
        let suggested_retry_secs = parsed_error
            .as_ref()
            .and_then(|w| w.error.details.as_ref())
            .and_then(|details| details.iter().find_map(|d| d.retry_delay.as_deref()))
            .and_then(|s| s.trim_end_matches('s').parse::<f64>().ok());

        last_error = Some(if is_resource_exhausted {
            AiEngineError::RateLimitExceeded
        } else {
            AiEngineError::ApiError {
                status: status.as_u16(),
                body: text,
                attempts: attempt + 1,
            }
        });

        if is_retryable && attempt + 1 < MAX_RETRIES {
            let backoff_secs = suggested_retry_secs
                .map(|s| s.ceil() as u64)
                .unwrap_or_else(|| 2u64.pow(attempt + 1)); // fallback: 2s, 4s, 8s
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            continue;
        }

        break;
    }

    // RateLimitExceeded nie dostaje dopisanego "(model przeciążony po N próbach)"
    // w treści - to osobny, jednoznaczny wariant, nie trzeba już tego odróżniać
    // przez string-matching (jak w usuniętym is_resource_exhausted_final).
    Err(last_error.unwrap_or(AiEngineError::ApiError {
        status: 0,
        body: "Nieznany błąd Gemini API".to_string(),
        attempts: MAX_RETRIES,
    }))
}

fn format_news_lines(news: &[NewsItem]) -> String {
    if news.is_empty() {
        return "(brak dopasowanych newsów w feedzie)".to_string();
    }
    news.iter()
        .map(|n| format!("- {}: {}", n.title, n.description))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generuje OBSZERNĄ, osobną analizę jednego instrumentu.
pub async fn generate_instrument_briefing(
    provider: &dyn AiProvider,
    instrument: &str,
    numeric_context: &str,
    news: &[NewsItem],
) -> Result<InstrumentBriefing, AiEngineError> {
    let news_lines = format_news_lines(news);

    let prompt = format!(
        "Jesteś doświadczonym analitykiem rynków finansowych analizującym instrument: {instrument}.\n\n\
         DANE LICZBOWE:\n{numeric_context}\n\n\
         NAJNOWSZE NEWSY Z PORTALU FINANSOWEGO (mogą być nieistotne - pomiń je wtedy):\n{news_lines}\n\n\
         Napisz OBSZERNĄ analizę po polsku (2-3 krótkie akapity, łącznie ok. 150-250 słów), \
         obejmującą:\n\
         1) Co obecnie dzieje się z {instrument} na podstawie danych liczbowych,\n\
         2) Czy newsy powyżej mają realny związek z {instrument}, i jeśli tak - jak mogą wpłynąć \
            na jego zachowanie w najbliższych sesjach,\n\
         3) Na co warto zwrócić uwagę / jakie jest ryzyko błędnej interpretacji tych danych.\n\
         Pisz prostym, konkretnym językiem, bez żargonu. Nie dodawaj nagłówków \
         markdown ani kodu - tylko czysty tekst podzielony na akapity."
    );

    let commentary = provider.generate(prompt).await?;

    Ok(InstrumentBriefing {
        instrument: instrument.to_string(),
        commentary,
    })
}

fn label_to_tv_ticker(label: &str) -> &'static str {
    match label {
        "NASDAQ" => "NASDAQ:IXIC",
        "SP500" => "SP:SPX",
        _ => "SP:SPX",
    }
}

pub fn find_strongest_pair(reports: &[AnalyticalReport]) -> Option<&AnalyticalReport> {
    reports.iter().max_by(|a, b| {
        a.correlation
            .abs()
            .partial_cmp(&b.correlation.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub fn generate_correlation_pine_script(equity_pair_symbol: &str) -> String {
    let parts: Vec<&str> = equity_pair_symbol.split("->").collect();
    let (leader_label, follower_label) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("NASDAQ", "SP500")
    };

    let leader_ticker = label_to_tv_ticker(leader_label);
    let follower_ticker = label_to_tv_ticker(follower_label);

    format!(
        r#"//@version=6
indicator("Trading Help: {leader_label}/{follower_label} Correlation", overlay=false)

lengthInput = input.int(20, title="Okno korelacji (świece)")
lagInput = input.int(1, title="Przesunięcie (lag, świece)")

leaderClose = request.security("{leader_ticker}", timeframe.period, close)
followerClose = request.security("{follower_ticker}", timeframe.period, close)
leaderShifted = leaderClose[lagInput]
correlation = ta.correlation(leaderShifted, followerClose, lengthInput)

plot(correlation, title="Korelacja {leader_label}->{follower_label}", color=color.aqua)
hline(0, "Zero", color=color.gray)
hline(0.5, "+0.5", color=color.green)
hline(-0.5, "-0.5", color=color.red)
"#,
        leader_label = leader_label,
        follower_label = follower_label,
        leader_ticker = leader_ticker,
        follower_ticker = follower_ticker,
    )
}

/// Stałe, ręcznie napisane wyjaśnienie skryptu korelacji (nie generowane przez AI).
pub fn explain_correlation_script(equity_pair_symbol: &str) -> String {
    let parts: Vec<&str> = equity_pair_symbol.split("->").collect();
    let (leader_label, follower_label) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("NASDAQ", "SP500")
    };

    format!(
        "Ten wskaźnik pokazuje, jak silnie {leader_label} 'przewiduje' ruch {follower_label} \
         z jednodniowym wyprzedzeniem. Linia korelacji porusza się w zakresie od -1 do +1: \
         wartości bliskie +1 oznaczają, że wzrost {leader_label} wczoraj zwykle poprzedza wzrost \
         {follower_label} dzisiaj; wartości bliskie -1 oznaczają zależność odwrotną; wartości \
         bliskie 0 oznaczają brak przewidywalnego związku.\n\n\
         Parametr 'Okno korelacji' (domyślnie 20 świec) to liczba dni branych pod uwagę przy \
         każdym przeliczeniu - mniejsza wartość daje bardziej czułą, ale bardziej 'szarpaną' \
         linię; większa wartość wygładza wykres, ale wolniej reaguje na zmiany.\n\n\
         Parametr 'Przesunięcie (lag)' określa, o ile sesji do przodu sprawdzamy wpływ - domyślnie \
         1, zgodnie z analizą w aplikacji. Oba parametry możesz swobodnie zmieniać w ustawieniach \
         wskaźnika w TradingView (ikona koła zębatego przy nazwie wskaźnika).",
        leader_label = leader_label,
        follower_label = follower_label,
    )
}

pub fn generate_gsr_pine_script() -> String {
    r#"//@version=6
indicator("Trading Help: Gold/Silver Ratio (GSR)", overlay=false)

maLengthInput = input.int(50, title="Okres średniej kroczącej")
highBandInput = input.float(80.0, title="Górne pasmo GSR")
lowBandInput = input.float(50.0, title="Dolne pasmo GSR")

gsr = request.security("TVC:GOLDSILVER", timeframe.period, close)
gsrMa = ta.sma(gsr, maLengthInput)

plot(gsr, title="GSR", color=color.yellow, linewidth=2)
plot(gsrMa, title="Średnia krocząca GSR", color=color.blue)
hline(highBandInput, "GSR wysoki", color=color.red)
hline(lowBandInput, "GSR niski", color=color.green)
"#
    .to_string()
}

/// Stałe, ręcznie napisane wyjaśnienie skryptu GSR (nie generowane przez AI).
pub fn explain_gsr_script() -> String {
    "Ten wskaźnik pokazuje relację Gold/Silver Ratio (GSR) - ile uncji srebra kosztuje jedna \
     uncja złota - bezpośrednio z wbudowanego w TradingView indeksu GOLDSILVER, więc nie musi \
     nic samodzielnie przeliczać.\n\n\
     Żółta linia to bieżąca wartość GSR, niebieska to jej średnia krocząca (domyślnie z 50 \
     świec) pokazująca długoterminowy trend bez dziennego 'szumu'.\n\n\
     Czerwona pozioma linia (domyślnie 80) oznacza historycznie wysoki poziom GSR - zwykle \
     interpretowany jako srebro relatywnie tanie względem złota. Zielona pozioma linia \
     (domyślnie 50) oznacza historycznie niski poziom - srebro relatywnie drogie względem \
     złota. Oba progi możesz dowolnie zmienić w ustawieniach wskaźnika, żeby dopasować je do \
     własnej analizy historycznej - to tylko orientacyjne wartości domyślne, nie sztywna reguła."
        .to_string()
}

#[cfg(test)]
mod find_strongest_pair_tests {
    use super::*;

    fn report(symbol: &str, correlation: f64) -> AnalyticalReport {
        AnalyticalReport {
            symbol: symbol.to_string(),
            correlation,
            volatility: 0.0,
            sentiment_impact: 0.0,
            timestamp: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn picks_report_with_highest_absolute_correlation() {
        let reports = vec![
            report("NASDAQ->SP500", 0.2),
            report("SP500->NASDAQ", 0.85),
            report("GOLD->SILVER", -0.4),
        ];

        let strongest = find_strongest_pair(&reports).expect("powinien znaleźć raport");
        assert_eq!(strongest.symbol, "SP500->NASDAQ");
    }

    #[test]
    fn negative_correlation_with_larger_magnitude_beats_smaller_positive() {
        let reports = vec![
            report("NASDAQ->SP500", 0.3),
            report("GOLD->SILVER", -0.9),
        ];

        let strongest = find_strongest_pair(&reports).expect("powinien znaleźć raport");
        assert_eq!(strongest.symbol, "GOLD->SILVER");
    }

    #[test]
    fn does_not_panic_when_correlation_is_nan() {
        let reports = vec![
            report("NASDAQ->SP500", f64::NAN),
            report("SP500->NASDAQ", 0.5),
        ];

        let result = find_strongest_pair(&reports);
        assert!(result.is_some());
    }

    #[test]
    fn returns_none_for_empty_reports() {
        let reports: Vec<AnalyticalReport> = vec![];
        assert!(find_strongest_pair(&reports).is_none());
    }
}
