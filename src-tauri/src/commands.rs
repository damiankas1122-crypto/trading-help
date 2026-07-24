// src-tauri/src/commands.rs
use crate::{models, market_engine, analysis_engine, ai_engine, news_engine, history_store, keychain};
use tauri::{AppHandle, Emitter};
use time::OffsetDateTime;
use std::time::Duration;
use thiserror::Error;

/// Typowane błędy warstwy komend Tauri. Zastępuje wcześniejsze Result<T, String>
/// przekazywane bezpośrednio z niższych warstw (market_engine, history_store, ai_engine),
/// co zmuszało do zgadywania rodzaju błędu po treści stringa. Publiczne komendy
/// #[tauri::command] nadal zwracają Result<T, String> do frontendu (kontrakt IPC
/// bez zmian) - stringifikacja dzieje się w jednym miejscu, na samym końcu.
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Błąd pobierania danych rynkowych: {0}")]
    MarketData(String),

    #[error(transparent)]
    Ai(#[from] ai_engine::AiEngineError),

    #[error("Błąd zapisu/odczytu lokalnej historii: {0}")]
    Storage(String),

    #[error("Błąd magazynu kluczy systemu: {0}")]
    Keychain(String),

    #[error("Brak danych do analizy indeksów")]
    NoStrongestPair,
}

#[tauri::command]
pub fn calculate_correlation(data_a: Vec<f64>, data_b: Vec<f64>) -> f64 {
    if data_a.is_empty() || data_b.is_empty() || data_a.len() != data_b.len() {
        return 0.0;
    }

    let n = data_a.len() as f64;
    let mean_a = data_a.iter().sum::<f64>() / n;
    let mean_b = data_b.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut sum_sq_a = 0.0;
    let mut sum_sq_b = 0.0;

    for i in 0..data_a.len() {
        let diff_a = data_a[i] - mean_a;
        let diff_b = data_b[i] - mean_b;
        numerator += diff_a * diff_b;
        sum_sq_a += diff_a * diff_a;
        sum_sq_b += diff_b * diff_b;
    }

    let denominator = (sum_sq_a * sum_sq_b).sqrt();
    if denominator == 0.0 {
        return 0.0;
    }
    numerator / denominator
}

fn to_returns(closes: &[f64]) -> Vec<f64> {
    closes
        .windows(2)
        .filter(|w| w[0] != 0.0)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect()
}

fn align_and_correlate_lagged(leader: &[f64], follower: &[f64], lag: usize) -> f64 {
    let len = leader.len().min(follower.len());
    if len <= lag + 1 {
        return 0.0;
    }
    let leader_tail = &leader[leader.len() - len..];
    let follower_tail = &follower[follower.len() - len..];
    let leader_slice = leader_tail[..len - lag].to_vec();
    let follower_slice = follower_tail[lag..].to_vec();
    calculate_correlation(leader_slice, follower_slice)
}

/// Buduje pojedynczy AnalyticalReport dla pary leader/follower.
/// Wydzielone z get_cross_market_analysis, żeby dało się to przetestować
/// bez sieci (get_cross_market_analysis_inner robi fetch z Yahoo Finance).
fn build_pair_report(
    leader_label: &str,
    leader_data: &[models::MarketData],
    leader_closes: &[f64],
    follower_label: &str,
    follower_closes: &[f64],
    timestamp: &str,
) -> models::AnalyticalReport {
    let leader_returns = to_returns(leader_closes);
    let follower_returns = to_returns(follower_closes);
    let correlation = align_and_correlate_lagged(&leader_returns, &follower_returns, DEFAULT_LAG);
    // Regresja: zmienność MUSI pochodzić z danych leadera, nie followera -
    // to był kiedyś zamieniony argument (bug w get_cross_market_analysis).
    let volatility = analysis_engine::calculate_volatility(leader_data);

    models::AnalyticalReport {
        symbol: format!("{}->{}", leader_label, follower_label),
        correlation,
        volatility,
        sentiment_impact: 0.0,
        timestamp: timestamp.to_string(),
    }
}

const DEFAULT_LAG: usize = 1;

async fn get_cross_market_analysis_inner() -> Result<Vec<models::AnalyticalReport>, CommandError> {
    let (nasdaq, sp500) = tokio::join!(
        market_engine::fetch_market_data("^IXIC"),
        market_engine::fetch_market_data("^GSPC"),
    );

    let nasdaq = nasdaq.map_err(CommandError::MarketData)?;
    let sp500 = sp500.map_err(CommandError::MarketData)?;

    let markets: Vec<(&str, &Vec<models::MarketData>, Vec<f64>)> = vec![
        ("NASDAQ", &nasdaq, nasdaq.iter().map(|d| d.close).collect()),
        ("SP500", &sp500, sp500.iter().map(|d| d.close).collect()),
    ];

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
    let mut reports = Vec::new();

    for (leader_label, leader_data, leader_closes) in &markets {
        for (follower_label, _follower_data, follower_closes) in &markets {
            if leader_label == follower_label {
                continue;
            }
            reports.push(build_pair_report(
                leader_label,
                leader_data,
                leader_closes,
                follower_label,
                follower_closes,
                &timestamp,
            ));
        }
    }

    Ok(reports)
}

#[tauri::command]
pub async fn get_cross_market_analysis() -> Result<Vec<models::AnalyticalReport>, String> {
    get_cross_market_analysis_inner()
        .await
        .map_err(|e| e.to_string())
}

async fn get_precious_metals_analysis_inner() -> Result<models::PreciousMetalsReport, CommandError> {
    let (gold, silver) = tokio::join!(
        market_engine::fetch_market_data("GC=F"),
        market_engine::fetch_market_data("SI=F"),
    );

    let gold = gold.map_err(CommandError::MarketData)?;
    let silver = silver.map_err(CommandError::MarketData)?;

    let gold_closes: Vec<f64> = gold.iter().map(|d| d.close).collect();
    let silver_closes: Vec<f64> = silver.iter().map(|d| d.close).collect();

    let gold_returns = to_returns(&gold_closes);
    let silver_returns = to_returns(&silver_closes);
    let correlation = align_and_correlate_lagged(&gold_returns, &silver_returns, 0);

    let gold_volatility = analysis_engine::calculate_volatility(&gold);
    let silver_volatility = analysis_engine::calculate_volatility(&silver);

    let current_gsr = match (gold_closes.last(), silver_closes.last()) {
        (Some(g), Some(s)) if *s != 0.0 => g / s,
        _ => 0.0,
    };
    let gsr_30d_ago = match (gold_closes.first(), silver_closes.first()) {
        (Some(g), Some(s)) if *s != 0.0 => g / s,
        _ => 0.0,
    };
    let gsr_change_pct = if gsr_30d_ago != 0.0 {
        ((current_gsr - gsr_30d_ago) / gsr_30d_ago) * 100.0
    } else {
        0.0
    };

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();

    Ok(models::PreciousMetalsReport {
        correlation,
        current_gsr,
        gsr_30d_ago,
        gsr_change_pct,
        gold_volatility,
        silver_volatility,
        timestamp,
    })
}

#[tauri::command]
pub async fn get_precious_metals_analysis() -> Result<models::PreciousMetalsReport, String> {
    get_precious_metals_analysis_inner()
        .await
        .map_err(|e| e.to_string())
}

fn numeric_context_for_equity(instrument: &str, reports: &[models::AnalyticalReport]) -> String {
    let leader_prefix = format!("{}->", instrument);
    reports
        .iter()
        .filter(|r| r.symbol.starts_with(&leader_prefix))
        .map(|r| format!("- {}: korelacja={:.4}, zmienność={:.4}", r.symbol, r.correlation, r.volatility))
        .collect::<Vec<_>>()
        .join("\n")
}

fn numeric_context_for_metal(metal: &str, report: &models::PreciousMetalsReport) -> String {
    let volatility = if metal == "GOLD" { report.gold_volatility } else { report.silver_volatility };
    format!(
        "- Korelacja Złoto-Srebro: {:.4}\n- Obecny GSR: {:.2}\n- GSR 30 dni temu: {:.2}\n- Zmiana GSR: {:.2}%\n- Zmienność {}: {:.4}",
        report.correlation, report.current_gsr, report.gsr_30d_ago, report.gsr_change_pct, metal, volatility
    )
}

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

async fn get_full_briefing_inner(app: AppHandle, slot: String) -> Result<models::FullBriefing, CommandError> {
    let equity_reports = get_cross_market_analysis_inner().await?;
    let metals_report = get_precious_metals_analysis_inner().await?;

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
            numeric_context_for_metal(instrument, metals_report)
        } else {
            numeric_context_for_equity(instrument, equity_reports)
        };

        numeric_context.push_str("\n\nZMIANA WZGLĘDEM POPRZEDNIEJ ANALIZY:\n");
        numeric_context.push_str(delta_context);

        (numeric_context, filtered_news)
    }

    let (ctx_nasdaq, news_nasdaq) = build_instrument_context("NASDAQ", &all_news, &equity_reports, &metals_report, &delta_context);
    let (ctx_sp500, news_sp500) = build_instrument_context("SP500", &all_news, &equity_reports, &metals_report, &delta_context);
    let (ctx_gold, news_gold) = build_instrument_context("GOLD", &all_news, &equity_reports, &metals_report, &delta_context);
    let (ctx_silver, news_silver) = build_instrument_context("SILVER", &all_news, &equity_reports, &metals_report, &delta_context);

    // Gemini free tier: limit 5 zapytań/minutę. Sekwencyjne wywołania z odstępem
    // zamiast 4 równoległych (tokio::join!) - wolniejsze, ale mieszczące się w limicie.
    // Każdy krok emituje event "briefing-progress", żeby frontend mógł pokazać postęp.
    const GEMINI_CALL_SPACING: Duration = Duration::from_secs(13);
    const TOTAL_INSTRUMENTS: u32 = 4;
  // Konstruujemy providera raz - dziś zawsze Gemini, ale to jest miejsce,
    // które w Etapie 6 wybierze dostawcę na podstawie ustawień użytkownika.
    let ai_provider: Box<dyn ai_engine::AiProvider> = Box::new(ai_engine::GeminiProvider);
    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "NASDAQ".to_string(),
        step: 1,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_nasdaq = ai_engine::generate_instrument_briefing(ai_provider.as_ref(), "NASDAQ", &ctx_nasdaq, &news_nasdaq).await?;
    tokio::time::sleep(GEMINI_CALL_SPACING).await;

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "SP500".to_string(),
        step: 2,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_sp500 = ai_engine::generate_instrument_briefing(ai_provider.as_ref(), "SP500", &ctx_sp500, &news_sp500).await?;
    tokio::time::sleep(GEMINI_CALL_SPACING).await;

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "GOLD".to_string(),
        step: 3,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_gold = ai_engine::generate_instrument_briefing(ai_provider.as_ref(), "GOLD", &ctx_gold, &news_gold).await?;
    tokio::time::sleep(GEMINI_CALL_SPACING).await;

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "SILVER".to_string(),
        step: 4,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_silver = ai_engine::generate_instrument_briefing(ai_provider.as_ref(), "SILVER", &ctx_silver, &news_silver).await?;

    let instrument_briefings = vec![
        briefing_nasdaq,
        briefing_sp500,
        briefing_gold,
        briefing_silver,
    ];

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

#[tauri::command]
pub async fn get_full_briefing(app: AppHandle, slot: String) -> Result<models::FullBriefing, String> {
    get_full_briefing_inner(app, slot)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_last_snapshot(app: AppHandle) -> Option<models::Snapshot> {
    history_store::load_last_snapshot(&app)
}

#[tauri::command]
pub fn save_gemini_api_key(key: String) -> Result<(), String> {
    keychain::save_gemini_api_key(&key).map_err(|e| CommandError::Keychain(e).to_string())
}

#[tauri::command]
pub fn has_gemini_api_key() -> bool {
    keychain::has_gemini_api_key()
}

#[tauri::command]
pub fn delete_gemini_api_key() -> Result<(), String> {
    keychain::delete_gemini_api_key().map_err(|e| CommandError::Keychain(e).to_string())
}

#[cfg(test)]
mod build_pair_report_tests {
    use super::*;

    fn candle(close: f64) -> models::MarketData {
        models::MarketData {
            symbol: "TEST".to_string(),
            time: "2026-01-01".to_string(),
            open: close,
            high: close,
            low: close,
            close,
        }
    }

    #[test]
    fn volatility_comes_from_leader_not_follower() {
        let leader_data = vec![candle(100.0), candle(110.0), candle(90.0), candle(105.0)];
        let follower_data = vec![candle(100.0), candle(100.5), candle(99.5), candle(100.0)];

        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();
        let follower_closes: Vec<f64> = follower_data.iter().map(|d| d.close).collect();

        let report = build_pair_report(
            "NASDAQ",
            &leader_data,
            &leader_closes,
            "SP500",
            &follower_closes,
            "2026-01-01",
        );

        let expected_leader_volatility = analysis_engine::calculate_volatility(&leader_data);
        let follower_volatility = analysis_engine::calculate_volatility(&follower_data);

        assert!((report.volatility - expected_leader_volatility).abs() < 1e-9);
        assert!((report.volatility - follower_volatility).abs() > 1e-9);
    }

    #[test]
    fn symbol_format_is_leader_arrow_follower() {
        let data = vec![candle(100.0), candle(101.0)];
        let closes: Vec<f64> = data.iter().map(|d| d.close).collect();

        let report = build_pair_report("GOLD", &data, &closes, "SILVER", &closes, "2026-01-01");
        assert_eq!(report.symbol, "GOLD->SILVER");
    }
}
