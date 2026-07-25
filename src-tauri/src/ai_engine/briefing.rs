//! Główna analiza jednego instrumentu: komentarz AI, sentyment newsów,
//! wybór wariantu Pine Script (`PineVariant`) i cytowania uzasadniające
//! konkretne zdania. Jedno wywołanie Gemini na instrument - wszystkie te
//! pola wracają w jednym ustrukturyzowanym JSON-ie (patrz komentarz w
//! prompt), żeby nie mnożyć wywołań API ponad potrzebę (rate-limit Gemini).

use crate::models::{Citation, InstrumentBriefing, NewsItem};
use super::{format_news_lines, label_to_tv_ticker, strip_json_fence, AiEngineError, AiProvider};
use serde::Deserialize;

#[derive(Deserialize)]
struct CitationResponse {
    claim: String,
    evidence_type: String,
    evidence_label: String,
}

#[derive(Deserialize)]
struct InstrumentBriefingResponse {
    commentary: String,
    sentiment_impact: f64,
    pine_variant: String,
    #[serde(default)]
    citations: Vec<CitationResponse>,
}

/// max cytowań - prompt prosi o tyle samo, to tylko zabezpieczenie w kodzie
const MAX_CITATIONS: usize = 5;

/// link do newsa nigdy od AI - szukamy dokładnego tytułu w liście z promptu,
/// brak dopasowania = cytowanie leci do kosza
fn resolve_citations(raw: Vec<CitationResponse>, news: &[NewsItem]) -> Vec<Citation> {
    raw.into_iter()
        .filter_map(|c| {
            if c.evidence_type == "news" {
                news.iter()
                    .find(|n| n.title.eq_ignore_ascii_case(c.evidence_label.trim()))
                    .map(|n| Citation {
                        claim: c.claim,
                        evidence_type: "news".to_string(),
                        evidence_label: n.title.clone(),
                        evidence_link: Some(n.link.clone()),
                    })
            } else {
                Some(Citation {
                    claim: c.claim,
                    evidence_type: "numeric".to_string(),
                    evidence_label: c.evidence_label,
                    evidence_link: None,
                })
            }
        })
        .take(MAX_CITATIONS)
        .collect()
}

/// AI wybiera tylko nazwę wariantu, nigdy nie pisze samego Pine Scripta -
/// kod zawsze się kompiluje, bo leci z gotowego szablonu
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PineVariant {
    Uptrend,
    Downtrend,
    Consolidation,
}

impl PineVariant {
    /// nieznana nazwa (literówka, halucynacja) ląduje bezpiecznie jako Consolidation
    fn from_ai_choice(s: &str) -> Self {
        match s {
            "trend_wzrostowy" => PineVariant::Uptrend,
            "trend_spadkowy" => PineVariant::Downtrend,
            _ => PineVariant::Consolidation,
        }
    }
}

/// analiza jednego instrumentu + sentyment newsów, jedno wywołanie Gemini
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
         WAŻNE OGRANICZENIE DANYCH: RSI i MACD powyżej to WYŁĄCZNIE bieżąca, pojedyncza migawka \
         (jeden punkt w czasie) - nie masz dostępu do ich wcześniejszych wartości ani historii. \
         NIE PISZ, że coś \"przecięło\", \"przebiło\" albo \"crossover\" linii sygnałowej/poziomu - \
         to zakładałoby wiedzę o tym, co działo się wcześniej, której nie posiadasz. Opisuj tylko \
         BIEŻĄCY stan, np. \"MACD znajduje się powyżej/poniżej linii sygnałowej\", \"RSI jest powyżej/ \
         poniżej poziomu 50\" - bez sugerowania świeżości czy kierunku zmiany tego stanu.\n\n\
         Odpowiedz WYŁĄCZNIE poprawnym obiektem JSON, bez markdown, bez bloków kodu, \
         bez żadnego tekstu przed ani po - dokładnie w tej postaci:\n\
         {{\"commentary\": \"...\", \"sentiment_impact\": 0.0, \"pine_variant\": \"...\", \
         \"citations\": [{{\"claim\": \"...\", \"evidence_type\": \"...\", \"evidence_label\": \"...\"}}]}}\n\n\
         Pole \"commentary\": OBSZERNA analiza po polsku (2-3 krótkie akapity, łącznie ok. \
         150-250 słów) jako pojedynczy string (znaki nowej linii jako \\n), obejmująca:\n\
         1) Co obecnie dzieje się z {instrument} na podstawie danych liczbowych,\n\
         2) Czy newsy powyżej mają realny związek z {instrument}, i jeśli tak - jak mogą wpłynąć \
            na jego zachowanie w najbliższych sesjach,\n\
         3) Na co warto zwrócić uwagę / jakie jest ryzyko błędnej interpretacji tych danych.\n\
         Pisz prostym, konkretnym językiem, bez żargonu, bez nagłówków markdown.\n\n\
         Pole \"sentiment_impact\": liczba zmiennoprzecinkowa od -1.0 do 1.0 opisująca wydźwięk \
         newsów powyżej DLA {instrument} - -1.0 to skrajnie negatywny, 1.0 skrajnie pozytywny, \
         0.0 gdy newsy są neutralne lub nieistotne dla {instrument}.\n\n\
         Pole \"pine_variant\": DOKŁADNIE jedna z trzech wartości: \"trend_wzrostowy\", \
         \"trend_spadkowy\" albo \"konsolidacja\" - wybierz na podstawie RSI i MACD z DANYCH \
         LICZBOWYCH powyżej. Wskazówka: RSI wyraźnie powyżej 50 razem z linią MACD powyżej linii \
         sygnałowej MACD sugeruje \"trend_wzrostowy\"; RSI wyraźnie poniżej 50 razem z linią MACD \
         poniżej linii sygnałowej sugeruje \"trend_spadkowy\"; w pozostałych przypadkach (sprzeczne \
         wskazania albo RSI blisko 50) wybierz \"konsolidacja\".\n\n\
         Pole \"citations\": lista 2-4 obiektów, każdy uzasadniający JEDNO konkretne \
         stwierdzenie z pola \"commentary\" - żeby user widział NA CZYM dokładnie oparte jest \
         dane zdanie, zamiast musieć ufać samemu tekstowi. Każdy obiekt ma:\n\
         - \"claim\": krótki fragment (kilka słów) z Twojej analizy, który ten dowód uzasadnia,\n\
         - \"evidence_type\": \"news\" jeśli dowodem jest jeden z newsów powyżej, albo \"numeric\" \
           jeśli dowodem jest jedna z DANYCH LICZBOWYCH powyżej,\n\
         - \"evidence_label\": dla \"news\" - DOKŁADNY tytuł newsa SKOPIOWANY z listy powyżej (nie \
           parafrazuj, musi być identyczny znak w znak, inaczej cytowanie zostanie odrzucone); dla \
           \"numeric\" - konkretna nazwa i wartość, np. \"RSI(14) = 63.2\" albo \"korelacja \
           {instrument}->SP500 = 0.18\".\n\
         Jeśli żadne konkretne stwierdzenie nie da się sensownie uzasadnić dostępnymi danymi, \
         zwróć pustą listę [] zamiast wymyślać dowód."
    );

    let raw_response = provider.generate(prompt).await?;
    let json_text = strip_json_fence(&raw_response);

    let parsed: InstrumentBriefingResponse = serde_json::from_str(json_text).map_err(|e| {
        AiEngineError::ResponseParseFailed(format!(
            "nie udało się sparsować JSON-a z sentymentem dla {instrument}: {e} (surowa odpowiedź: {json_text})"
        ))
    })?;

    let variant = PineVariant::from_ai_choice(&parsed.pine_variant);
    let citations = resolve_citations(parsed.citations, news);

    Ok(InstrumentBriefing {
        instrument: instrument.to_string(),
        commentary: parsed.commentary,
        sentiment_impact: parsed.sentiment_impact.clamp(-1.0, 1.0),
        pine_script_signal: generate_signal_pine_script(instrument, variant),
        pine_script_signal_explanation: explain_signal_pine_script(instrument, variant),
        citations,
    })
}

fn pine_script_uptrend(instrument: &str) -> String {
    let ticker = label_to_tv_ticker(instrument);
    format!(
        r#"//@version=6
indicator("Trading Help: {instrument} - Sygnał trendu wzrostowego", overlay=false)

rsiLengthInput = input.int(14, title="Okres RSI")
rsiThresholdInput = input.float(50.0, title="Próg RSI dla sygnału wzrostowego")

price = request.security("{ticker}", timeframe.period, close)
rsiValue = ta.rsi(price, rsiLengthInput)
[macdLine, signalLine, _] = ta.macd(price, 12, 26, 9)

bullishSignal = rsiValue > rsiThresholdInput and macdLine > signalLine

plot(rsiValue, title="RSI", color=bullishSignal ? color.green : color.gray, linewidth=2)
hline(rsiThresholdInput, "Próg sygnału", color=color.blue)
hline(70, "Wykupienie", color=color.red)
hline(30, "Wyprzedanie", color=color.green)
"#,
        instrument = instrument,
        ticker = ticker,
    )
}

fn pine_script_downtrend(instrument: &str) -> String {
    let ticker = label_to_tv_ticker(instrument);
    format!(
        r#"//@version=6
indicator("Trading Help: {instrument} - Sygnał trendu spadkowego", overlay=false)

rsiLengthInput = input.int(14, title="Okres RSI")
rsiThresholdInput = input.float(50.0, title="Próg RSI dla sygnału spadkowego")

price = request.security("{ticker}", timeframe.period, close)
rsiValue = ta.rsi(price, rsiLengthInput)
[macdLine, signalLine, _] = ta.macd(price, 12, 26, 9)

bearishSignal = rsiValue < rsiThresholdInput and macdLine < signalLine

plot(rsiValue, title="RSI", color=bearishSignal ? color.red : color.gray, linewidth=2)
hline(rsiThresholdInput, "Próg sygnału", color=color.blue)
hline(70, "Wykupienie", color=color.red)
hline(30, "Wyprzedanie", color=color.green)
"#,
        instrument = instrument,
        ticker = ticker,
    )
}

fn pine_script_consolidation(instrument: &str) -> String {
    let ticker = label_to_tv_ticker(instrument);
    format!(
        r#"//@version=6
indicator("Trading Help: {instrument} - Konsolidacja", overlay=false)

rsiLengthInput = input.int(14, title="Okres RSI")

price = request.security("{ticker}", timeframe.period, close)
rsiValue = ta.rsi(price, rsiLengthInput)

plot(rsiValue, title="RSI", color=color.gray, linewidth=2)
hline(70, "Wykupienie", color=color.red)
hline(50, "Środek zakresu", color=color.blue)
hline(30, "Wyprzedanie", color=color.green)
"#,
        instrument = instrument,
        ticker = ticker,
    )
}

/// renderuje szablon dla wariantu wybranego przez AI, samo nic nie generuje
pub fn generate_signal_pine_script(instrument: &str, variant: PineVariant) -> String {
    match variant {
        PineVariant::Uptrend => pine_script_uptrend(instrument),
        PineVariant::Downtrend => pine_script_downtrend(instrument),
        PineVariant::Consolidation => pine_script_consolidation(instrument),
    }
}

/// wyjaśnienie skryptu, na sztywno - nie generowane przez AI
pub fn explain_signal_pine_script(instrument: &str, variant: PineVariant) -> String {
    let base = format!(
        "Ten wskaźnik dla {instrument} pokazuje RSI(14) w osobnym panelu, z progami 70 \
         (wykupienie) i 30 (wyprzedanie)."
    );
    match variant {
        PineVariant::Uptrend => format!(
            "{base} AI oceniło obecną sytuację jako trend wzrostowy - linia RSI zmienia kolor na \
             zielony, gdy RSI jest powyżej progu sygnału ORAZ linia MACD jest powyżej linii \
             sygnałowej MACD, czyli gdy oba wskaźniki potwierdzają się nawzajem. Sam zielony kolor \
             nie jest rekomendacją zakupu, tylko wizualnym potwierdzeniem tych dwóch warunków naraz."
        ),
        PineVariant::Downtrend => format!(
            "{base} AI oceniło obecną sytuację jako trend spadkowy - linia RSI zmienia kolor na \
             czerwony, gdy RSI jest poniżej progu sygnału ORAZ linia MACD jest poniżej linii \
             sygnałowej MACD. Sam czerwony kolor nie jest rekomendacją sprzedaży, tylko wizualnym \
             potwierdzeniem tych dwóch warunków naraz."
        ),
        PineVariant::Consolidation => format!(
            "{base} AI oceniło obecną sytuację jako konsolidację (brak wyraźnego trendu) - \
             wskaźnik pokazuje sam RSI bez dodatkowego oznaczania sygnałów kierunkowych, żeby nie \
             sugerować kierunku, którego obecnie nie widać w danych."
        ),
    }
}

#[cfg(test)]
mod pine_variant_tests {
    use super::*;

    #[test]
    fn from_ai_choice_maps_known_strings() {
        assert_eq!(PineVariant::from_ai_choice("trend_wzrostowy"), PineVariant::Uptrend);
        assert_eq!(PineVariant::from_ai_choice("trend_spadkowy"), PineVariant::Downtrend);
        assert_eq!(PineVariant::from_ai_choice("konsolidacja"), PineVariant::Consolidation);
    }

    #[test]
    fn from_ai_choice_defaults_unknown_strings_to_consolidation() {
        // to jest cała "walidacja" - halucynacja nie może wybrać nieistniejącego wariantu
        assert_eq!(PineVariant::from_ai_choice("cokolwiek innego"), PineVariant::Consolidation);
        assert_eq!(PineVariant::from_ai_choice(""), PineVariant::Consolidation);
    }

    fn assert_well_formed_pine_script(code: &str) {
        assert!(code.trim_start().starts_with("//@version=6"));
        assert!(code.contains("indicator("));
        let open = code.matches('(').count();
        let close = code.matches(')').count();
        assert_eq!(open, close, "niezbalansowane nawiasy okrągłe w:\n{code}");
    }

    #[test]
    fn all_variants_generate_well_formed_pine_script_for_each_instrument() {
        for variant in [PineVariant::Uptrend, PineVariant::Downtrend, PineVariant::Consolidation] {
            for instrument in ["NASDAQ", "SP500", "GOLD", "SILVER"] {
                assert_well_formed_pine_script(&generate_signal_pine_script(instrument, variant));
            }
        }
    }

    #[test]
    fn gold_and_silver_use_dedicated_tickers_not_the_sp500_fallback() {
        assert!(generate_signal_pine_script("GOLD", PineVariant::Consolidation).contains("TVC:GOLD"));
        assert!(generate_signal_pine_script("SILVER", PineVariant::Consolidation).contains("TVC:SILVER"));
    }
}

#[cfg(test)]
mod citation_tests {
    use super::*;

    fn news(title: &str, link: &str) -> NewsItem {
        NewsItem {
            title: title.to_string(),
            description: "opis".to_string(),
            link: link.to_string(),
            published: "2026-01-01".to_string(),
        }
    }

    fn raw(claim: &str, evidence_type: &str, evidence_label: &str) -> CitationResponse {
        CitationResponse {
            claim: claim.to_string(),
            evidence_type: evidence_type.to_string(),
            evidence_label: evidence_label.to_string(),
        }
    }

    #[test]
    fn news_citation_gets_real_link_on_exact_title_match() {
        let items = vec![news("Fed podnosi stopy", "https://example.com/fed")];
        let result = resolve_citations(vec![raw("stopy w górę", "news", "Fed podnosi stopy")], &items);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].evidence_link, Some("https://example.com/fed".to_string()));
    }

    #[test]
    fn news_citation_is_dropped_when_title_does_not_match_any_real_news() {
        // cała ochrona przed zmyślonym linkiem - tytułu nie ma w promptcie, więc nie ufamy
        let items = vec![news("Fed podnosi stopy", "https://example.com/fed")];
        let result = resolve_citations(vec![raw("coś", "news", "Zupełnie inny tytuł")], &items);

        assert!(result.is_empty());
    }

    #[test]
    fn numeric_citation_passes_through_without_a_link() {
        let result = resolve_citations(vec![raw("RSI wysoki", "numeric", "RSI(14) = 63.2")], &[]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].evidence_type, "numeric");
        assert_eq!(result[0].evidence_link, None);
    }

    #[test]
    fn citations_are_capped_at_max_citations() {
        let raws: Vec<CitationResponse> = (0..10)
            .map(|i| raw(&format!("claim {i}"), "numeric", &format!("label {i}")))
            .collect();

        let result = resolve_citations(raws, &[]);
        assert_eq!(result.len(), MAX_CITATIONS);
    }
}
