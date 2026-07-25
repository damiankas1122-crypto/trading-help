//! Pełny briefing (4 instrumenty + Pine Scripty korelacji/GSR) na start
//! sesji. Zależy od cross_market/precious_metals (dane liczbowe), ai_engine
//! (treść komentarzy, Pine Script per instrument) i history_store
//! (porównanie z poprzednią sesją, guard na niezmienione dane). Zero
//! #[tauri::command] tutaj - publiczny wrapper żyje w commands/mod.rs.

use crate::{models, ai_engine, news_engine, history_store};
use tauri::{AppHandle, Emitter};
use time::OffsetDateTime;
use std::time::Duration;
use super::cross_market;
use super::precious_metals;
use super::error::CommandError;

fn build_delta_context(
    current_equity: &[models::AnalyticalReport],
    current_metals: &models::PreciousMetalsReport,
    previous: &Option<models::Snapshot>,
) -> String {
    match previous {
        None => "Brak poprzedniej analizy do porównania - to pierwsze uruchomienie aplikacji.".to_string(),
        Some(prev) => {
            let mut lines = vec![format!(
                "Poprzednia analiza: {} (porównujemy z tamtym momentem)",
                prev.slot
            )];

            for curr in current_equity {
                if let Some(prev_r) = prev.equity_reports.iter().find(|p| p.symbol == curr.symbol) {
                    let diff = curr.correlation - prev_r.correlation;
                    lines.push(format!(
                        "{}: korelacja zmieniła się o {:+.4} (z {:.4} na {:.4})",
                        curr.symbol, diff, prev_r.correlation, curr.correlation
                    ));
                }
            }

            let gsr_diff = current_metals.current_gsr - prev.metals_report.current_gsr;
            lines.push(format!(
                "GSR zmienił się o {:+.2} (z {:.2} na {:.2})",
                gsr_diff, prev.metals_report.current_gsr, current_metals.current_gsr
            ));

            lines.join("\n")
        }
    }
}

fn market_data_unchanged(
    current_equity: &[models::AnalyticalReport],
    current_metals: &models::PreciousMetalsReport,
    previous: &models::Snapshot,
) -> bool {
    const EPS: f64 = 1e-9;

    if current_equity.len() != previous.equity_reports.len() {
        return false;
    }

    for curr in current_equity {
        match previous.equity_reports.iter().find(|p| p.symbol == curr.symbol) {
            Some(prev_r) => {
                if (curr.correlation - prev_r.correlation).abs() > EPS
                    || (curr.volatility - prev_r.volatility).abs() > EPS
                {
                    return false;
                }
            }
            None => return false,
        }
    }

    if (current_metals.correlation - previous.metals_report.correlation).abs() > EPS
        || (current_metals.current_gsr - previous.metals_report.current_gsr).abs() > EPS
    {
        return false;
    }

    true
}

const INSTRUMENTS: [&str; 4] = ["NASDAQ", "SP500", "GOLD", "SILVER"];
// Gemini free tier: 5/min, więc leci sekwencyjnie z odstępem zamiast 4 na
// raz - wolniej, ale się mieści. Każdy krok emituje "briefing-progress" dla UI.
const GEMINI_CALL_SPACING: Duration = Duration::from_secs(13);

fn build_instrument_context(
    instrument: &str,
    all_news: &[models::NewsItem],
    equity_reports: &[models::AnalyticalReport],
    metals_report: &models::PreciousMetalsReport,
    delta_context: &str,
) -> (String, Vec<models::NewsItem>) {
    let keywords = news_engine::keywords_for(instrument);
    let filtered_news = news_engine::filter_news_for_instrument(all_news, keywords, 5);

    let mut numeric_context = if instrument == "GOLD" || instrument == "SILVER" {
        precious_metals::numeric_context_for_metal(instrument, metals_report)
    } else {
        cross_market::numeric_context_for_equity(instrument, equity_reports)
    };

    numeric_context.push_str("\n\nZMIANA WZGLĘDEM POPRZEDNIEJ ANALIZY:\n");
    numeric_context.push_str(delta_context);

    (numeric_context, filtered_news)
}

pub(crate) async fn get_full_briefing_inner(app: AppHandle, slot: String) -> Result<models::FullBriefing, CommandError> {
    let equity_reports = cross_market::get_cross_market_analysis_inner().await?;
    let metals_report = precious_metals::get_precious_metals_analysis_inner().await?;

    let previous_snapshot = history_store::load_last_snapshot(&app);

    if let Some(prev) = &previous_snapshot {
        if market_data_unchanged(&equity_reports, &metals_report, prev) {
            let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
            let refreshed_snapshot = models::Snapshot {
                equity_reports: equity_reports.clone(),
                metals_report: metals_report.clone(),
                timestamp,
                slot: slot.clone(),
            };
            history_store::save_snapshot(&app, &refreshed_snapshot).map_err(CommandError::Storage)?;

            return Ok(models::FullBriefing {
                slot,
                compared_to: Some(prev.slot.clone()),
                equity_reports,
                metals_report,
                instrument_briefings: vec![],
                pine_script_correlation: String::new(),
                pine_script_correlation_explanation: String::new(),
                pine_script_gsr: String::new(),
                pine_script_gsr_explanation: String::new(),
                is_stale_data: true,
                stale_data_message: Some(format!(
                    "Brak nowych danych rynkowych od ostatniej analizy ({}). Yahoo Finance nie opublikował jeszcze nowej świecy dziennej — spróbuj ponownie po otwarciu kolejnej sesji handlowej.",
                    prev.slot
                )),
            });
        }
    }

    let all_news = news_engine::fetch_all_news().await.unwrap_or_default();
    let delta_context = build_delta_context(&equity_reports, &metals_report, &previous_snapshot);
    let compared_to = previous_snapshot.as_ref().map(|s| s.slot.clone());

    // provider raz na start - dziś zawsze Gemini, w Etapie 6 tu będzie wybór z ustawień
    let ai_provider: Box<dyn ai_engine::AiProvider> = Box::new(ai_engine::GeminiProvider);
    let mut instrument_briefings = Vec::with_capacity(INSTRUMENTS.len());

    for (i, instrument) in INSTRUMENTS.iter().enumerate() {
        let (ctx, news) = build_instrument_context(instrument, &all_news, &equity_reports, &metals_report, &delta_context);

        let _ = app.emit("briefing-progress", models::BriefingProgress {
            instrument: instrument.to_string(),
            step: (i + 1) as u32,
            total: INSTRUMENTS.len() as u32,
        });

        let briefing = ai_engine::generate_instrument_briefing(ai_provider.as_ref(), instrument, &ctx, &news).await?;
        instrument_briefings.push(briefing);

        if i + 1 < INSTRUMENTS.len() {
            tokio::time::sleep(GEMINI_CALL_SPACING).await;
        }
    }

    let strongest_equity = ai_engine::find_strongest_pair(&equity_reports)
        .ok_or(CommandError::NoStrongestPair)?;

    let pine_script_correlation = ai_engine::generate_correlation_pine_script(&strongest_equity.symbol);
    let pine_script_correlation_explanation = ai_engine::explain_correlation_script(&strongest_equity.symbol);
    let pine_script_gsr = ai_engine::generate_gsr_pine_script();
    let pine_script_gsr_explanation = ai_engine::explain_gsr_script();

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();

    let new_snapshot = models::Snapshot {
        equity_reports: equity_reports.clone(),
        metals_report: metals_report.clone(),
        timestamp,
        slot: slot.clone(),
    };
    history_store::save_snapshot(&app, &new_snapshot).map_err(CommandError::Storage)?;

    Ok(models::FullBriefing {
        slot,
        compared_to,
        equity_reports,
        metals_report,
        instrument_briefings,
        pine_script_correlation,
        pine_script_correlation_explanation,
        pine_script_gsr,
        pine_script_gsr_explanation,
        is_stale_data: false,
        stale_data_message: None,
    })
}
