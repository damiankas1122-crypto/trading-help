//! On-demand AI briefing for a single instrument: one Gemini call, no loop over
//! the watchlist, and no correlation or GSR in the prompt context - those form a
//! separate passive panel computed in `market_context.rs`. No #[tauri::command]
//! here.

use crate::{models, market_engine, analysis_engine, ai_engine, news_engine};
use super::error::CommandError;
use super::instruments::{is_supported, yahoo_symbol_for};

/// Only the instrument's own data: no correlation with anything else, so the
/// commentary stays within what was actually asked for.
fn numeric_context_for_instrument(instrument: &str, data: &[models::MarketData]) -> String {
    let technicals = analysis_engine::calculate_technicals(data);
    let volatility = analysis_engine::calculate_volatility(data);
    let latest_close = data.last().map(|d| d.close).unwrap_or(0.0);
    format!(
        "- {instrument}: cena={:.2}, zmienność={:.4}, RSI(14)={:.2}, MACD={:.4} (sygnał={:.4})",
        latest_close, volatility, technicals.rsi, technicals.macd_line, technicals.macd_signal
    )
}

pub(crate) async fn get_instrument_briefing_inner(
    instrument: String,
    operation_id: String,
) -> Result<models::InstrumentBriefing, CommandError> {
    // The guard removes the registry entry on every exit path, including a panic.
    let (cancel_token, _guard) = ai_engine::cancel::register(&operation_id);

    if !is_supported(&instrument) {
        return Err(CommandError::UnknownInstrument(instrument));
    }

    let symbol = yahoo_symbol_for(&instrument)
        .ok_or_else(|| CommandError::UnknownInstrument(instrument.clone()))?;
    let data = market_engine::fetch_market_data(symbol)
        .await
        .map_err(CommandError::MarketData)?;
    let numeric_context = numeric_context_for_instrument(&instrument, &data);

    // An RSS failure differs from "no news for this instrument"; None carries
    // that distinction into the prompt.
    let filtered_news = match news_engine::fetch_all_news().await {
        Ok(all_news) => match news_engine::news_source_for(&instrument) {
            news_engine::NewsSource::Keywords(keywords) => {
                Some(news_engine::filter_news_for_instrument(&all_news, keywords, 5))
            }
            // The prompt cannot express "no source assigned" yet: it takes
            // Option<&[NewsItem]>, where None already means the feed failed.
            // Carrying the third state further means changing the prompt, the
            // model and the frontend together, so it waits for the instrument
            // panel - no sourceless instrument is reachable from the UI today.
            news_engine::NewsSource::Unassigned => Some(Vec::new()),
        },
        Err(e) => {
            log::warn!("Failed to fetch news for {instrument}: {e}");
            None
        }
    };

    let ai_provider: Box<dyn ai_engine::AiProvider> =
        Box::new(ai_engine::GeminiProvider::new(ai_engine::CallContext::new(cancel_token)));
    let briefing = ai_engine::generate_instrument_briefing(
        ai_provider.as_ref(),
        &instrument,
        &numeric_context,
        filtered_news.as_deref(),
    )
    .await?;

    Ok(briefing)
}
