//! Generowanie taktyki tradingowej NA ŻĄDANIE użytkownika (osobne wywołanie
//! AI, poza sekwencją 4 automatycznych calli briefingu - patrz
//! commands/tactics.rs). Scenariusz bull/bear/neutral z uzasadnieniem i
//! poziomami % względem bieżącej ceny, zawsze z dołączonym stałym
//! disclaimerem (nie generowanym przez AI).

use crate::models::{NewsItem, TradingTactic};
use super::{format_news_lines, strip_json_fence, AiEngineError, AiProvider};
use serde::Deserialize;

#[derive(Deserialize)]
struct TacticResponse {
    scenario: String,
    reasoning: String,
    target_pct: f64,
    stop_loss_pct: f64,
}

/// disclaimer do każdej taktyki, na sztywno - prawny tekst nie może pisać AI
const TACTIC_DISCLAIMER: &str = "To NIE jest porada inwestycyjna. Scenariusz i poziomy poniżej to \
     orientacyjna interpretacja danych liczbowych przez model AI, wygenerowana automatycznie i \
     obarczona ryzykiem błędu - może być całkowicie nietrafna. Nie podejmuj decyzji inwestycyjnych \
     wyłącznie na tej podstawie.";

const MAX_LEVEL_PCT: f64 = 20.0;

/// nieznana wartość ląduje bezpiecznie jako "neutral", tak jak PineVariant na Consolidation
fn normalize_scenario(s: &str) -> &'static str {
    match s {
        "bull" => "bull",
        "bear" => "bear",
        _ => "neutral",
    }
}

/// taktyka na żądanie - osobne wywołanie AI, poza sekwencją 4 calli briefingu
pub async fn generate_trading_tactic(
    provider: &dyn AiProvider,
    instrument: &str,
    numeric_context: &str,
    news: &[NewsItem],
    reference_price: f64,
) -> Result<TradingTactic, AiEngineError> {
    let news_lines = format_news_lines(news);

    let prompt = format!(
        "Jesteś doświadczonym analitykiem rynków finansowych. Na podstawie poniższych danych \
         zaproponuj JEDEN scenariusz tradingowy dla instrumentu {instrument}.\n\n\
         DANE LICZBOWE:\n{numeric_context}\n\n\
         NAJNOWSZE NEWSY (mogą być nieistotne - pomiń je wtedy):\n{news_lines}\n\n\
         WAŻNE OGRANICZENIE DANYCH: RSI i MACD powyżej to WYŁĄCZNIE bieżąca, pojedyncza migawka \
         (jeden punkt w czasie) - nie masz dostępu do ich wcześniejszych wartości ani historii. \
         NIE PISZ, że coś \"przecięło\", \"przebiło\" albo \"crossover\" linii sygnałowej/poziomu - \
         to zakładałoby wiedzę o tym, co działo się wcześniej, której nie posiadasz. Opisuj tylko \
         BIEŻĄCY stan, np. \"MACD znajduje się powyżej/poniżej linii sygnałowej\", \"RSI jest powyżej/ \
         poniżej poziomu 50\" - bez sugerowania świeżości czy kierunku zmiany tego stanu.\n\n\
         Odpowiedz WYŁĄCZNIE poprawnym obiektem JSON, bez markdown, bez bloków kodu, \
         bez żadnego tekstu przed ani po - dokładnie w tej postaci:\n\
         {{\"scenario\": \"...\", \"reasoning\": \"...\", \"target_pct\": 0.0, \"stop_loss_pct\": 0.0}}\n\n\
         Pole \"scenario\": DOKŁADNIE jedna z trzech wartości: \"bull\", \"bear\" albo \"neutral\".\n\n\
         Pole \"reasoning\": uzasadnienie po polsku (3-5 zdań, ok. 60-100 słów) odwołujące się \
         KONKRETNIE do podanych danych liczbowych (korelacja, RSI, MACD, sentyment newsów) - \
         dlaczego wybrałeś ten scenariusz, nie ogólniki. Wejście zawsze następuje po bieżącej cenie \
         (nie proponuj innego poziomu wejścia).\n\n\
         Pola \"target_pct\", \"stop_loss_pct\": liczby zmiennoprzecinkowe, procentowe przesunięcie \
         względem BIEŻĄCEJ ceny (nie realna cena) - target_pct dodatnie dla \"bull\" / ujemne dla \
         \"bear\", stop_loss_pct odwrotny znak niż target_pct i mniejsza wartość bezwzględna (ryzyko \
         mniejsze niż potencjalny zysk). Dla \"neutral\" ustaw obie wartości blisko 0.0. Wartości \
         bezwzględne nie powinny przekraczać {MAX_LEVEL_PCT}."
    );

    let raw_response = provider.generate(prompt).await?;
    let json_text = strip_json_fence(&raw_response);

    let parsed: TacticResponse = serde_json::from_str(json_text).map_err(|e| {
        AiEngineError::ResponseParseFailed(format!(
            "nie udało się sparsować JSON-a z taktyką dla {instrument}: {e} (surowa odpowiedź: {json_text})"
        ))
    })?;

    Ok(TradingTactic {
        instrument: instrument.to_string(),
        scenario: normalize_scenario(&parsed.scenario).to_string(),
        reasoning: parsed.reasoning,
        // zawsze 0.0, wejście po bieżącej cenie - kiedyś o to pytaliśmy AI, ale
        // zawsze wychodziło 0.0, więc olaliśmy pytanie
        entry_pct: 0.0,
        target_pct: parsed.target_pct.clamp(-MAX_LEVEL_PCT, MAX_LEVEL_PCT),
        stop_loss_pct: parsed.stop_loss_pct.clamp(-MAX_LEVEL_PCT, MAX_LEVEL_PCT),
        reference_price,
        disclaimer: TACTIC_DISCLAIMER.to_string(),
        timestamp: time::OffsetDateTime::now_utc().unix_timestamp().to_string(),
    })
}

#[cfg(test)]
mod trading_tactic_tests {
    use super::*;

    #[test]
    fn normalize_scenario_maps_known_strings() {
        assert_eq!(normalize_scenario("bull"), "bull");
        assert_eq!(normalize_scenario("bear"), "bear");
        assert_eq!(normalize_scenario("neutral"), "neutral");
    }

    #[test]
    fn normalize_scenario_defaults_unknown_strings_to_neutral() {
        // halucynacja nie może wybrać nieistniejącego scenariusza - ląduje na "neutral"
        assert_eq!(normalize_scenario("byczy"), "neutral");
        assert_eq!(normalize_scenario(""), "neutral");
    }
}
