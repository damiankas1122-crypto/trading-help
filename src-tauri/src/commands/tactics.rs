//! On-demand trading tactic plus its automatic, immutable verification (24h/7d)
//! and aggregated accuracy statistics. Depends on cross_market/precious_metals
//! for numeric context, ai_engine for generation, and tactic_store/tactic_engine
//! for persistence and scoring. No #[tauri::command] here.

use crate::{models, market_engine, ai_engine, news_engine, tactic_engine, tactic_store};
use tauri::AppHandle;
use time::OffsetDateTime;
use super::cross_market;
use super::precious_metals;
use super::error::CommandError;
use super::instruments::{is_metal, is_supported, yahoo_symbol_for};

/// On-demand tactic, separate from briefings. Persists to tactic_store for later
/// verification.
pub(crate) async fn generate_trading_tactic_inner(app: &AppHandle, instrument: String) -> Result<models::TradingTactic, CommandError> {
    if !is_supported(&instrument) {
        return Err(CommandError::UnknownInstrument(instrument));
    }

    // Fetch only the side of the market the prompt actually uses; a GOLD tactic
    // previously also pulled NASDAQ and SP500 and then ignored them.
    let numeric_context = if is_metal(&instrument) {
        let metals_report = precious_metals::get_precious_metals_analysis_inner().await?;
        precious_metals::numeric_context_for_metal(&instrument, &metals_report)
    } else {
        let equity_reports = cross_market::get_cross_market_analysis_inner().await?;
        cross_market::numeric_context_for_equity(&instrument, &equity_reports)
    };

    // An RSS failure differs from "no news for this instrument"; None carries
    // that distinction into the prompt.
    let filtered_news = match news_engine::fetch_all_news().await {
        Ok(all_news) => {
            let keywords = news_engine::keywords_for(&instrument);
            Some(news_engine::filter_news_for_instrument(&all_news, keywords, 5))
        }
        Err(e) => {
            eprintln!("Failed to fetch news for {instrument}: {e}");
            None
        }
    };

    let price_history = market_engine::fetch_market_data(yahoo_symbol_for(&instrument))
        .await
        .map_err(CommandError::MarketData)?;
    let reference_price = price_history.last().map(|d| d.close).unwrap_or(0.0);

    let ai_provider: Box<dyn ai_engine::AiProvider> = Box::new(ai_engine::GeminiProvider);
    let tactic = ai_engine::generate_trading_tactic(
        ai_provider.as_ref(),
        &instrument,
        &numeric_context,
        filtered_news.as_deref(),
        reference_price,
    )
    .await?;

    let generated_at = OffsetDateTime::now_utc().unix_timestamp();
    let tracked = models::TrackedTactic {
        id: format!("{}-{}", tactic.instrument, generated_at),
        instrument: tactic.instrument.clone(),
        scenario: tactic.scenario.clone(),
        reference_price: tactic.reference_price,
        entry_pct: tactic.entry_pct,
        target_pct: tactic.target_pct,
        stop_loss_pct: tactic.stop_loss_pct,
        generated_at,
        verified_24h: None,
        verified_7d: None,
    };
    // A failed store write still returns the tactic; it just will not count
    // towards the statistics.
    let _ = tactic_store::append(app, tracked);

    Ok(tactic)
}

/// Verifies tactics after 24h/7d and computes the statistics. "Automatic" means
/// on every call of this command, since the app does not run in the background.
pub(crate) async fn get_tactic_track_record_inner(app: &AppHandle) -> Result<models::TacticTrackRecord, CommandError> {
    let mut tactics = tactic_store::load_all(app);
    let now = OffsetDateTime::now_utc().unix_timestamp();

    // Per-call price cache: several tactics on one instrument share a single fetch.
    let mut price_cache: std::collections::HashMap<&'static str, Vec<models::MarketData>> = std::collections::HashMap::new();

    for tactic in tactics.iter_mut() {
        let needs_24h = tactic.verified_24h.is_none() && now >= tactic.generated_at + tactic_engine::HOUR_24_SECONDS;
        let needs_7d = tactic.verified_7d.is_none() && now >= tactic.generated_at + tactic_engine::DAYS_7_SECONDS;
        if !needs_24h && !needs_7d {
            continue;
        }

        let symbol = yahoo_symbol_for(&tactic.instrument);
        if !price_cache.contains_key(symbol) {
            if let Ok(data) = market_engine::fetch_market_data(symbol).await {
                price_cache.insert(symbol, data);
            }
        }
        let Some(history) = price_cache.get(symbol) else { continue };

        if needs_24h {
            let until = tactic.generated_at + tactic_engine::HOUR_24_SECONDS;
            if let Some(outcome) = tactic_engine::evaluate_outcome(tactic, history, until) {
                tactic.verified_24h = Some(models::TacticVerification {
                    outcome: outcome.to_string(),
                    checked_at: now,
                });
            }
        }
        if needs_7d {
            let until = tactic.generated_at + tactic_engine::DAYS_7_SECONDS;
            if let Some(outcome) = tactic_engine::evaluate_outcome(tactic, history, until) {
                tactic.verified_7d = Some(models::TacticVerification {
                    outcome: outcome.to_string(),
                    checked_at: now,
                });
            }
        }
    }

    tactic_store::save_all(app, &tactics).map_err(CommandError::Storage)?;
    Ok(tactic_engine::compute_track_record(&tactics))
}
