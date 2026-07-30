//! Precious metals analysis: Au-Ag correlation, Gold/Silver Ratio (current and
//! 30 days back), volatility and technical indicators for both metals. Uses the
//! correlation maths from `cross_market.rs` rather than keeping its own copy.
//! No #[tauri::command] here.

use crate::{models, market_engine, analysis_engine};
use time::OffsetDateTime;
use super::cross_market::{
    align_and_correlate_lagged, daily_change_pct, dated_returns, overlapping_observations,
};
use super::error::CommandError;

/// Close of the candle nearest `target_unix`. Since the window grew to 90 days,
/// the first candle can no longer be assumed to be "30 days ago".
fn close_nearest_to(data: &[models::MarketData], target_unix: i64) -> Option<f64> {
    data.iter()
        .min_by_key(|d| (d.timestamp - target_unix).abs())
        .map(|d| d.close)
}

pub(crate) async fn get_precious_metals_analysis_inner() -> Result<models::PreciousMetalsReport, CommandError> {
    let (gold, silver) = tokio::join!(
        market_engine::fetch_market_data("GC=F"),
        market_engine::fetch_market_data("SI=F"),
    );

    let gold = gold.map_err(CommandError::MarketData)?;
    let silver = silver.map_err(CommandError::MarketData)?;

    Ok(build_metals_report(&gold, &silver, OffsetDateTime::now_utc()))
}

/// The metals report; extracted so it can be tested without network access.
/// `now` is a parameter because the 30-day GSR reference point depends on it.
fn build_metals_report(
    gold: &[models::MarketData],
    silver: &[models::MarketData],
    now: OffsetDateTime,
) -> models::PreciousMetalsReport {
    let gold_closes: Vec<f64> = gold.iter().map(|d| d.close).collect();
    let silver_closes: Vec<f64> = silver.iter().map(|d| d.close).collect();

    // Returns come from the candles so the session dates reach the join.
    let gold_returns = dated_returns(gold);
    let silver_returns = dated_returns(silver);
    let correlation = align_and_correlate_lagged(&gold_returns, &silver_returns, 0);
    let overlap = overlapping_observations(&gold_returns, &silver_returns);

    let gold_volatility = analysis_engine::calculate_volatility(gold);
    let silver_volatility = analysis_engine::calculate_volatility(silver);
    let gold_technicals = analysis_engine::calculate_technicals(gold);
    let silver_technicals = analysis_engine::calculate_technicals(silver);

    let current_gsr = match (gold_closes.last(), silver_closes.last()) {
        (Some(g), Some(s)) if *s != 0.0 => g / s,
        _ => 0.0,
    };
    let thirty_days_ago_unix = (now - time::Duration::days(30)).unix_timestamp();
    let gsr_30d_ago = match (
        close_nearest_to(gold, thirty_days_ago_unix),
        close_nearest_to(silver, thirty_days_ago_unix),
    ) {
        (Some(g), Some(s)) if s != 0.0 => g / s,
        _ => 0.0,
    };
    let gsr_change_pct = if gsr_30d_ago != 0.0 {
        ((current_gsr - gsr_30d_ago) / gsr_30d_ago) * 100.0
    } else {
        0.0
    };

    let timestamp = now.unix_timestamp().to_string();
    let gold_price = gold_closes.last().copied().unwrap_or(0.0);
    let silver_price = silver_closes.last().copied().unwrap_or(0.0);

    models::PreciousMetalsReport {
        correlation,
        overlapping_observations: overlap,
        current_gsr,
        gsr_30d_ago,
        gsr_change_pct,
        gold_volatility,
        silver_volatility,
        gold_technicals,
        silver_technicals,
        gold_price,
        silver_price,
        gold_daily_change_pct: daily_change_pct(&gold_closes),
        silver_daily_change_pct: daily_change_pct(&silver_closes),
        timestamp,
    }
}

pub(crate) fn numeric_context_for_metal(metal: &str, report: &models::PreciousMetalsReport) -> String {
    let (volatility, technicals) = if metal == "GOLD" {
        (report.gold_volatility, &report.gold_technicals)
    } else {
        (report.silver_volatility, &report.silver_technicals)
    };
    let mut lines: Vec<String> = Vec::new();

    if let Some(value) = report.correlation {
        lines.push(format!(
            "- Korelacja Złoto-Srebro: {:.4} (wspólne sesje: {})",
            value, report.overlapping_observations
        ));
    }

    // Only the correlation depends on the date join. The GSR is a ratio of the two
    // latest closes, so it stays measured when the join fails, and it is still a
    // statement about the pair - which is why the "Złoto-Srebro" label keeps its
    // footing here. Do not strip it for symmetry with `cross_market.rs`: there the
    // label went because nothing about the follower survived, and here the ratio
    // and its 30-day change do.
    lines.push(format!("- Obecny GSR (Złoto-Srebro): {:.2}", report.current_gsr));
    lines.push(format!("- GSR 30 dni temu: {:.2}", report.gsr_30d_ago));
    lines.push(format!("- Zmiana GSR: {:.2}%", report.gsr_change_pct));
    lines.push(format!("- Zmienność {}: {:.4}", metal, volatility));
    lines.push(format!("- RSI(14) {}: {:.2}", metal, technicals.rsi));
    lines.push(format!(
        "- MACD {}: {:.4} (sygnał={:.4})",
        metal, technicals.macd_line, technicals.macd_signal
    ));

    lines.join("\n")
}

#[cfg(test)]
mod metals_report_tests {
    use super::*;

    fn candle(day_offset: i64, close: f64) -> models::MarketData {
        let date = time::macros::date!(2026 - 01 - 05) + time::Duration::days(day_offset);
        models::MarketData {
            symbol: "TEST".to_string(),
            time: date.to_string(),
            timestamp: date.midnight().assume_utc().unix_timestamp(),
            open: close,
            high: close,
            low: close,
            close,
        }
    }

    fn gold_close(d: i64) -> f64 {
        2000.0 + d as f64 + (d % 7) as f64 * 5.0
    }

    fn silver_close(d: i64) -> f64 {
        25.0 + d as f64 * 0.1 + (d % 5) as f64 * 0.5
    }

    fn series(days: impl IntoIterator<Item = i64>, close: impl Fn(i64) -> f64) -> Vec<models::MarketData> {
        days.into_iter().map(|d| candle(d, close(d))).collect()
    }

    fn now_at(day_offset: i64) -> OffsetDateTime {
        (time::macros::date!(2026 - 01 - 05) + time::Duration::days(day_offset))
            .midnight()
            .assume_utc()
    }

    #[test]
    fn partially_disjoint_calendars_measure_only_the_shared_sessions() {
        let gold = series(0..60, gold_close);
        // Silver misses every tenth session; six gold days have no counterpart.
        let silver = series((0..60).filter(|d| d % 10 != 3), silver_close);

        let report = build_metals_report(&gold, &silver, now_at(60));

        assert!(report.correlation.is_some());
        assert_eq!(report.overlapping_observations, 53);
        assert_eq!(report.gold_price, gold_close(59));
        assert_eq!(report.silver_price, silver_close(59));
    }

    #[test]
    fn disjoint_calendars_leave_the_gsr_and_both_metals_intact() {
        let gold = series(0..40, gold_close);
        let silver = series(200..240, silver_close);

        let report = build_metals_report(&gold, &silver, now_at(40));

        assert_eq!(report.correlation, None);
        assert_eq!(report.overlapping_observations, 0);

        // The GSR is a ratio of the latest closes, not a product of the join.
        assert_eq!(report.gold_price, gold_close(39));
        assert_eq!(report.silver_price, silver_close(239));
        assert_eq!(report.current_gsr, gold_close(39) / silver_close(239));
        assert!(report.current_gsr > 0.0);

        // Nearest candle to (now - 30d) on each side; the two series need not agree.
        assert_eq!(report.gsr_30d_ago, gold_close(10) / silver_close(200));
        assert!(report.gsr_change_pct != 0.0);

        // Volatility and technicals are per-metal and never touch the join.
        assert_eq!(report.gold_volatility, analysis_engine::calculate_volatility(&gold));
        assert_eq!(report.silver_volatility, analysis_engine::calculate_volatility(&silver));
        assert!(report.gold_volatility > 0.0);
        assert!(report.silver_volatility > 0.0);
        assert_eq!(
            report.gold_technicals.rsi,
            analysis_engine::calculate_technicals(&gold).rsi
        );
        assert_eq!(
            report.silver_technicals.macd_line,
            analysis_engine::calculate_technicals(&silver).macd_line
        );
    }

    fn report_with(correlation: Option<f64>, overlap: usize) -> models::PreciousMetalsReport {
        models::PreciousMetalsReport {
            correlation,
            overlapping_observations: overlap,
            current_gsr: 82.13,
            gsr_30d_ago: 79.40,
            gsr_change_pct: 3.44,
            gold_volatility: 0.0123,
            silver_volatility: 0.0234,
            gold_technicals: models::TechnicalIndicators {
                rsi: 55.25,
                macd_line: 1.5,
                macd_signal: 0.5,
            },
            silver_technicals: models::TechnicalIndicators {
                rsi: 44.75,
                macd_line: -1.5,
                macd_signal: -0.5,
            },
            gold_price: 2100.0,
            silver_price: 25.57,
            gold_daily_change_pct: 0.4,
            silver_daily_change_pct: -0.3,
            timestamp: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn prompt_context_carries_coefficient_and_sample_size() {
        let context = numeric_context_for_metal("GOLD", &report_with(Some(0.8231), 41));

        assert!(context.contains("Korelacja Złoto-Srebro: 0.8231"), "otrzymano {context}");
        assert!(context.contains("wspólne sesje: 41"), "otrzymano {context}");
        assert!(context.contains("82.13"), "otrzymano {context}");
    }

    #[test]
    fn unmeasured_correlation_removes_only_itself_and_leaves_the_gsr() {
        let context = numeric_context_for_metal("GOLD", &report_with(None, 4));

        let lowered = context.to_lowercase();
        assert!(!lowered.contains("korelacj"), "otrzymano {context}");
        assert!(!context.contains("0.0000"), "otrzymano {context}");
        assert!(!context.contains("n/d"), "otrzymano {context}");

        // The GSR survives a failed join, and so does the pair label attached to it.
        assert!(context.contains("Złoto-Srebro"), "otrzymano {context}");
        assert!(context.contains("82.13"), "otrzymano {context}");
        assert!(context.contains("79.40"), "otrzymano {context}");
        assert!(context.contains("3.44%"), "otrzymano {context}");
        assert!(context.contains("RSI(14) GOLD: 55.25"), "otrzymano {context}");
    }
}
