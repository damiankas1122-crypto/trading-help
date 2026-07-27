//! Briefing AI dla JEDNEGO instrumentu na żądanie (przycisk "Analizuj
//! {instrument}") - jedno wywołanie Gemini, bez pętli po watchliście i bez
//! korelacji/GSR w kontekście (to osobny, pasywny panel liczony w
//! `market_context.rs` - patrz CLAUDE.md, decyzja z sesji 2026-07-26). Zero
//! #[tauri::command] tutaj.

use crate::{models, market_engine, analysis_engine, ai_engine, news_engine};
use super::error::CommandError;

const VALID_INSTRUMENTS: [&str; 4] = ["NASDAQ", "SP500", "GOLD", "SILVER"];

fn yahoo_symbol_for(instrument: &str) -> &'static str {
    match instrument {
        "NASDAQ" => "^IXIC",
        "SP500" => "^GSPC",
        "GOLD" => "GC=F",
        "SILVER" => "SI=F",
        _ => "^GSPC",
    }
}

/// tylko dane WŁASNE instrumentu - żadnej korelacji z innym instrumentem,
/// żeby AI nie wplatało w komentarz kontekstu spoza tego, co user faktycznie
/// poprosił o przeanalizowanie
fn numeric_context_for_instrument(instrument: &str, data: &[models::MarketData]) -> String {
    let technicals = analysis_engine::calculate_technicals(data);
    let volatility = analysis_engine::calculate_volatility(data);
    let latest_close = data.last().map(|d| d.close).unwrap_or(0.0);
    format!(
        "- {instrument}: cena={:.2}, zmienność={:.4}, RSI(14)={:.2}, MACD={:.4} (sygnał={:.4})",
        latest_close, volatility, technicals.rsi, technicals.macd_line, technicals.macd_signal
    )
}

pub(crate) async fn get_instrument_briefing_inner(instrument: String) -> Result<models::InstrumentBriefing, CommandError> {
    if !VALID_INSTRUMENTS.contains(&instrument.as_str()) {
        return Err(CommandError::UnknownInstrument(instrument));
    }

    let data = market_engine::fetch_market_data(yahoo_symbol_for(&instrument))
        .await
        .map_err(CommandError::MarketData)?;
    let numeric_context = numeric_context_for_instrument(&instrument, &data);

    let all_news = news_engine::fetch_all_news().await.unwrap_or_default();
    let keywords = news_engine::keywords_for(&instrument);
    let filtered_news = news_engine::filter_news_for_instrument(&all_news, keywords, 5);

    let ai_provider: Box<dyn ai_engine::AiProvider> = Box::new(ai_engine::GeminiProvider);
    let briefing = ai_engine::generate_instrument_briefing(
        ai_provider.as_ref(),
        &instrument,
        &numeric_context,
        &filtered_news,
    )
    .await?;

    Ok(briefing)
}
