//! Numeric market data (correlations, GSR, templated Pine Scripts): no AI, no
//! rate limit. Refreshed independently of per-instrument briefings (see
//! `instrument_briefing.rs`); feeds the ticker tape and the market context
//! panel. No #[tauri::command] here.

use crate::{models, history_store, ai_engine, catalog};
use tauri::AppHandle;
use time::OffsetDateTime;
use super::cross_market;
use super::precious_metals;
use super::error::CommandError;

/// Splits a "LEADER->FOLLOWER" report symbol back into catalogue entries.
fn resolve_pair(pair_symbol: &str) -> Option<(&'static catalog::Instrument, &'static catalog::Instrument)> {
    let (leader, follower) = pair_symbol.split_once("->")?;
    Some((catalog::find(leader.trim())?, catalog::find(follower.trim())?))
}

/// Picks the pair worth a correlation script, or `None` when there is nothing
/// to plot. Both failures degrade instead of propagating: the correlation
/// script is one part of the context, and losing it must not take prices, GSR
/// and metals down with it.
///
/// The two dead ends mean different things and are logged separately - one is
/// the state of the market, the other a defect in this program.
fn correlation_script_pair(
    equity_reports: &[models::AnalyticalReport],
) -> Option<(&'static catalog::Instrument, &'static catalog::Instrument)> {
    let strongest = match ai_engine::find_strongest_equity_pair(equity_reports) {
        Some(report) => report,
        None => {
            log::warn!(
                "No correlation Pine Script: none of the {} equity reports carried a measurable \
                 correlation (too few overlapping sessions).",
                equity_reports.len()
            );
            return None;
        }
    };

    // Both sides must be catalogued before a script is emitted; an unresolvable
    // pair used to render as NASDAQ/SP500, handing the user a correct-looking
    // script about a market they did not ask about.
    match resolve_pair(&strongest.symbol) {
        Some(pair) => Some(pair),
        None => {
            log::warn!(
                "Internal inconsistency: the analysis produced pair '{}', which the catalogue does \
                 not know. No correlation Pine Script; the rest of the context is unaffected.",
                strongest.symbol
            );
            None
        }
    }
}

/// The script and its explanation, or a pair of `None`s when no correlation was
/// measurable. `unzip` on a single `Option` is what makes "both or neither"
/// structural rather than a rule to remember.
///
/// This is the testability boundary: everything below is pure and driven by the
/// reports alone, everything above needs the network and an `AppHandle`.
fn correlation_scripts(
    equity_reports: &[models::AnalyticalReport],
) -> (Option<String>, Option<String>) {
    correlation_script_pair(equity_reports)
        .map(|(leader, follower)| {
            (
                ai_engine::generate_correlation_pine_script(leader, follower),
                ai_engine::explain_correlation_script(leader, follower),
            )
        })
        .unzip()
}

pub(crate) async fn get_market_context_inner(app: AppHandle) -> Result<models::MarketContext, CommandError> {
    let equity_reports = cross_market::get_cross_market_analysis_inner().await?;
    let metals_report = precious_metals::get_precious_metals_analysis_inner().await?;

    let (pine_script_correlation, pine_script_correlation_explanation) =
        correlation_scripts(&equity_reports);

    let pine_script_gsr = ai_engine::generate_gsr_pine_script();
    let pine_script_gsr_explanation = ai_engine::explain_gsr_script();

    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();

    let snapshot = models::Snapshot {
        equity_reports: equity_reports.clone(),
        metals_report: metals_report.clone(),
        timestamp: timestamp.clone(),
    };
    history_store::save_snapshot(&app, &snapshot).map_err(CommandError::Storage)?;

    Ok(models::MarketContext {
        equity_reports,
        metals_report,
        pine_script_correlation,
        pine_script_correlation_explanation,
        pine_script_gsr,
        pine_script_gsr_explanation,
        timestamp,
    })
}

#[cfg(test)]
mod correlation_scripts_tests {
    use super::*;

    fn report(symbol: &str, correlation: Option<f64>) -> models::AnalyticalReport {
        models::AnalyticalReport {
            symbol: symbol.to_string(),
            correlation,
            overlapping_observations: correlation.map_or(0, |_| 40),
            volatility: 0.0,
            technicals: models::TechnicalIndicators { rsi: 50.0, macd_line: 0.0, macd_signal: 0.0 },
            latest_close: 0.0,
            daily_change_pct: 0.0,
            timestamp: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn no_measurable_correlation_costs_the_script_and_nothing_else() {
        let reports = vec![
            report("NASDAQ->SP500", None),
            report("SP500->NASDAQ", None),
        ];

        assert_eq!(correlation_scripts(&reports), (None, None));
    }

    #[test]
    fn nan_correlation_does_not_count_as_a_measurement() {
        // Separate class from `None`: measured, but not comparable, so it must
        // not reach a script that would plot it as a real reading.
        let reports = vec![
            report("NASDAQ->SP500", Some(f64::NAN)),
            report("SP500->NASDAQ", Some(f64::NAN)),
        ];

        assert_eq!(correlation_scripts(&reports), (None, None));
    }

    #[test]
    fn measured_pair_is_scripted_for_the_instruments_it_names() {
        let reports = vec![report("NASDAQ->SP500", Some(0.82))];

        let (script, explanation) = correlation_scripts(&reports);

        // The invariant `unzip` is here to enforce: never one without the other.
        assert_eq!(script.is_some(), explanation.is_some());

        let script = script.expect("a measured pair must produce a script");
        let explanation = explanation.expect("a script is never emitted without its explanation");

        // Asserting on the tickers rather than the whole template: a full-text
        // comparison breaks on every cosmetic edit and teaches deleting the
        // assertion instead of reading it.
        let nasdaq = catalog::find("NASDAQ").expect("NASDAQ is catalogued");
        let sp500 = catalog::find("SP500").expect("SP500 is catalogued");
        assert!(script.contains(nasdaq.tv_ticker), "script must plot {}", nasdaq.tv_ticker);
        assert!(script.contains(sp500.tv_ticker), "script must plot {}", sp500.tv_ticker);
        assert!(explanation.contains(nasdaq.label), "explanation must name the pair it describes");
        assert!(explanation.contains(sp500.label), "explanation must name the pair it describes");
    }

    #[test]
    fn pair_the_catalogue_does_not_know_degrades_instead_of_failing() {
        // An unresolvable pair is an internal inconsistency, but the context is
        // still worth delivering: no script, no error, no panic.
        let reports = vec![report("FOO->BAR", Some(0.9))];

        assert_eq!(correlation_scripts(&reports), (None, None));
    }
}
