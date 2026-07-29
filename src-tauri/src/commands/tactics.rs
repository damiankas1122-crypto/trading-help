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

/// Write-path guard using the same predicate the scoring path applies, so
/// "storable" and "scorable" cannot drift apart (see `is_scorable_price`).
fn ensure_storable_price(instrument: &str, price: f64) -> Result<(), CommandError> {
    if tactic_engine::is_scorable_price(price) {
        return Ok(());
    }
    Err(CommandError::InvalidReferencePrice {
        instrument: instrument.to_string(),
        price,
    })
}

/// On-demand tactic, separate from briefings. Persists to tactic_store for later
/// verification.
pub(crate) async fn generate_trading_tactic_inner(
    app: &AppHandle,
    instrument: String,
    operation_id: String,
) -> Result<models::TradingTactic, CommandError> {
    // The guard removes the registry entry on every exit path, including a panic.
    let (cancel_token, _guard) = ai_engine::cancel::register(&operation_id);

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
        Ok(all_news) => match news_engine::news_source_for(&instrument) {
            news_engine::NewsSource::Keywords(keywords) => {
                Some(news_engine::filter_news_for_instrument(&all_news, keywords, 5))
            }
            // See instrument_briefing.rs: the prompt has no way to say "no source
            // assigned" yet, and that state is unreachable from the UI today.
            news_engine::NewsSource::Unassigned => Some(Vec::new()),
        },
        Err(e) => {
            log::warn!("Failed to fetch news for {instrument}: {e}");
            None
        }
    };

    let symbol = yahoo_symbol_for(&instrument)
        .ok_or_else(|| CommandError::UnknownInstrument(instrument.clone()))?;
    let price_history = market_engine::fetch_market_data(symbol)
        .await
        .map_err(CommandError::MarketData)?;
    // Second barrier behind the validation in market_engine, because this path
    // writes unreproducible data: a tactic stored with reference_price = 0.0
    // makes every target price 0.0 too, so evaluate_outcome scores it as a hit
    // forever and inflates the track record. Checked before the Gemini call, so
    // a broken price costs no quota.
    let reference_price = price_history.last().map(|d| d.close).unwrap_or(0.0);
    ensure_storable_price(&instrument, reference_price)?;

    let ai_provider: Box<dyn ai_engine::AiProvider> =
        Box::new(ai_engine::GeminiProvider::new(ai_engine::CallContext::new(cancel_token)));
    let tactic = ai_engine::generate_trading_tactic(
        ai_provider.as_ref(),
        &instrument,
        &numeric_context,
        filtered_news.as_deref(),
        reference_price,
    )
    .await?;

    // The price the model echoed back is what actually gets stored and scored,
    // so it is checked again rather than trusting the value sent into the prompt.
    ensure_storable_price(&tactic.instrument, tactic.reference_price)?;

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
    // This command runs every time the statistics are opened, so without this
    // flag the most frequent path would also rewrite the file most often.
    let mut changed = false;

    // Per-call price cache: several tactics on one instrument share a single fetch.
    let mut price_cache: std::collections::HashMap<&'static str, Vec<models::MarketData>> = std::collections::HashMap::new();

    for tactic in tactics.iter_mut() {
        let needs_24h = tactic.verified_24h.is_none() && now >= tactic.generated_at + tactic_engine::HOUR_24_SECONDS;
        let needs_7d = tactic.verified_7d.is_none() && now >= tactic.generated_at + tactic_engine::DAYS_7_SECONDS;
        if !needs_24h && !needs_7d {
            continue;
        }

        // Stored tactics may name an instrument no longer in the catalogue.
        // Skipping leaves them unverified, which beats scoring them against
        // another instrument's prices.
        let Some(symbol) = yahoo_symbol_for(&tactic.instrument) else {
            log::warn!("Skipping verification of unknown instrument: {}", tactic.instrument);
            continue;
        };
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
                changed = true;
            }
        }
        if needs_7d {
            let until = tactic.generated_at + tactic_engine::DAYS_7_SECONDS;
            if let Some(outcome) = tactic_engine::evaluate_outcome(tactic, history, until) {
                tactic.verified_7d = Some(models::TacticVerification {
                    outcome: outcome.to_string(),
                    checked_at: now,
                });
                changed = true;
            }
        }
    }

    if changed {
        tactic_store::save_all(app, &tactics).map_err(CommandError::Storage)?;
    }
    Ok(tactic_engine::compute_track_record(&tactics))
}

#[cfg(test)]
mod reference_price_tests {
    use super::*;

    #[test]
    fn zero_price_is_never_storable() {
        let err = ensure_storable_price("GOLD", 0.0).unwrap_err();
        assert!(matches!(err, CommandError::InvalidReferencePrice { .. }));
    }

    #[test]
    fn negative_and_non_finite_prices_are_rejected() {
        for price in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(
                ensure_storable_price("GOLD", price).is_err(),
                "cena {price} nie powinna być zapisywalna"
            );
        }
    }

    #[test]
    fn ordinary_price_passes() {
        assert!(ensure_storable_price("GOLD", 2410.5).is_ok());
    }
}
