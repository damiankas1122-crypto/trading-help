// src-tauri/src/commands.rs
use crate::{models, market_engine, analysis_engine, ai_engine, news_engine, history_store, keychain};
use tauri::{AppHandle, Emitter};
use time::OffsetDateTime;
use std::time::Duration;  

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

const DEFAULT_LAG: usize = 1;

#[tauri::command]
pub async fn get_cross_market_analysis() -> Result<Vec<models::AnalyticalReport>, String> {
    let (nasdaq, sp500) = tokio::join!(
        market_engine::fetch_market_data("^IXIC"),
        market_engine::fetch_market_data("^GSPC"),
    );

    let nasdaq = nasdaq?;
    let sp500 = sp500?;

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
            let leader_returns = to_returns(leader_closes);
            let follower_returns = to_returns(follower_closes);
            let correlation = align_and_correlate_lagged(&leader_returns, &follower_returns, DEFAULT_LAG);
            let volatility = analysis_engine::calculate_volatility(leader_data);

            reports.push(models::AnalyticalReport {
                symbol: format!("{}->{}", leader_label, follower_label),
                correlation,
                volatility,
                sentiment_impact: 0.0,
                timestamp: timestamp.clone(),
            });
        }
    }

    Ok(reports)
}

#[tauri::command]
pub async fn get_precious_metals_analysis() -> Result<models::PreciousMetalsReport, String> {
    let (gold, silver) = tokio::join!(
        market_engine::fetch_market_data("GC=F"),
        market_engine::fetch_market_data("SI=F"),
    );

    let gold = gold?;
    let silver = silver?;

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

#[tauri::command]
pub async fn get_full_briefing(app: AppHandle, slot: String) -> Result<models::FullBriefing, String> {
    let equity_reports = get_cross_market_analysis().await?;
    let metals_report = get_precious_metals_analysis().await?;

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
            history_store::save_snapshot(&app, &refreshed_snapshot)?;

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

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "NASDAQ".to_string(),
        step: 1,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_nasdaq = ai_engine::generate_instrument_briefing("NASDAQ", &ctx_nasdaq, &news_nasdaq).await?;
    tokio::time::sleep(GEMINI_CALL_SPACING).await;

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "SP500".to_string(),
        step: 2,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_sp500 = ai_engine::generate_instrument_briefing("SP500", &ctx_sp500, &news_sp500).await?;
    tokio::time::sleep(GEMINI_CALL_SPACING).await;

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "GOLD".to_string(),
        step: 3,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_gold = ai_engine::generate_instrument_briefing("GOLD", &ctx_gold, &news_gold).await?;
    tokio::time::sleep(GEMINI_CALL_SPACING).await;

    let _ = app.emit("briefing-progress", models::BriefingProgress {
        instrument: "SILVER".to_string(),
        step: 4,
        total: TOTAL_INSTRUMENTS,
    });
    let briefing_silver = ai_engine::generate_instrument_briefing("SILVER", &ctx_silver, &news_silver).await?;

    let instrument_briefings = vec![
        briefing_nasdaq,
        briefing_sp500,
        briefing_gold,
        briefing_silver,
    ];

    let strongest_equity = ai_engine::find_strongest_pair(&equity_reports)
        .ok_or_else(|| "Brak danych do analizy indeksów".to_string())?;

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
    history_store::save_snapshot(&app, &new_snapshot)?;

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
pub fn get_last_snapshot(app: AppHandle) -> Option<models::Snapshot> {
    history_store::load_last_snapshot(&app)
}

#[tauri::command]
pub fn save_gemini_api_key(key: String) -> Result<(), String> {
    keychain::save_gemini_api_key(&key)
}

#[tauri::command]
pub fn has_gemini_api_key() -> bool {
    keychain::has_gemini_api_key()
}

#[tauri::command]
pub fn delete_gemini_api_key() -> Result<(), String> {
    keychain::delete_gemini_api_key()
}