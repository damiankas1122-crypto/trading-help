//! Equity correlation (NASDAQ<->SP500): data fetch, lagged correlation,
//! volatility and technical indicators per leader/follower pair. The correlation
//! maths (`calculate_correlation`, `dated_returns`, `join_on_date`,
//! `align_and_correlate_lagged`) lives here but is shared with
//! `precious_metals.rs` and `custom_pair.rs`, hence `pub(crate)` rather than
//! private. No #[tauri::command] here - those live in commands/mod.rs.

use crate::{models, market_engine, analysis_engine};
use std::collections::HashMap;
use time::{Date, OffsetDateTime};
use super::error::CommandError;

pub(crate) fn calculate_correlation(data_a: &[f64], data_b: &[f64]) -> f64 {
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

/// Returns keyed by session date. A plain alias rather than a struct: it never
/// crosses the IPC boundary and carries no behaviour of its own.
pub(crate) type DatedSeries = Vec<(Date, f64)>;

/// Percentage returns between consecutive candles, attributed to the date of the
/// later candle.
///
/// The key is the UTC date, not the raw timestamp: Yahoo stamps a candle with the
/// session open, and exchanges open at different hours, so joining on `i64` would
/// never match across venues.
pub(crate) fn dated_returns(data: &[models::MarketData]) -> DatedSeries {
    let mut series: DatedSeries = Vec::with_capacity(data.len().saturating_sub(1));

    for window in data.windows(2) {
        let (previous, current) = (&window[0], &window[1]);
        // A zero previous close has no meaningful return; dropping the single
        // observation cannot shift the rest, because the join runs on dates.
        if previous.close == 0.0 {
            continue;
        }
        let Ok(moment) = OffsetDateTime::from_unix_timestamp(current.timestamp) else {
            continue;
        };
        series.push((moment.date(), (current.close - previous.close) / previous.close));
    }

    series.sort_by_key(|(date, _)| *date);
    // Stable sort keeps input order within a date, so on a duplicate the later
    // occurrence wins.
    series.dedup_by(|later, earlier| {
        if later.0 == earlier.0 {
            earlier.1 = later.1;
            true
        } else {
            false
        }
    });

    series
}

/// Observations whose dates appear in both series, ascending by date.
///
/// Missing days are dropped, never forward-filled: carrying the last known price
/// into a gap invents a 0% return, which understates volatility and overstates
/// correlation - it biases the output towards looking more confident than the
/// data supports.
pub(crate) fn join_on_date(a: &DatedSeries, b: &DatedSeries) -> (Vec<f64>, Vec<f64>) {
    let index_b: HashMap<Date, f64> = b.iter().copied().collect();

    let mut matched: Vec<(Date, f64, f64)> = a
        .iter()
        .filter_map(|(date, value_a)| index_b.get(date).map(|value_b| (*date, *value_a, *value_b)))
        .collect();
    matched.sort_by_key(|(date, _, _)| *date);

    matched.into_iter().map(|(_, value_a, value_b)| (value_a, value_b)).unzip()
}

pub(crate) fn overlapping_observations(a: &DatedSeries, b: &DatedSeries) -> usize {
    let index_b: HashMap<Date, f64> = b.iter().copied().collect();
    a.iter().filter(|(date, _)| index_b.contains_key(date)).count()
}

/// Lagged correlation over the dates the two series share.
///
/// The lag is applied after the join, so it shifts by one shared session rather
/// than by one array index - with mismatched trading calendars those are not the
/// same thing.
///
/// `None` means "not measured": 0.0 is a valid reading (no linear relationship)
/// and must not double as a failure code. NaN is not used as a sentinel either.
pub(crate) fn align_and_correlate_lagged(
    leader: &DatedSeries,
    follower: &DatedSeries,
    lag: usize,
) -> Option<f64> {
    let (leader_values, follower_values) = join_on_date(leader, follower);
    if leader_values.len() <= lag {
        return None;
    }

    let paired = leader_values.len() - lag;
    // Same threshold as RSI/MACD, so a report cannot claim "not enough data" for
    // the indicators and still print a correlation figure.
    if paired < analysis_engine::MIN_CLOSES_FOR_INDICATORS {
        return None;
    }

    Some(calculate_correlation(&leader_values[..paired], &follower_values[lag..]))
}

const DEFAULT_LAG: usize = 1;

pub(crate) fn daily_change_pct(closes: &[f64]) -> f64 {
    if closes.len() < 2 {
        return 0.0;
    }
    let prev = closes[closes.len() - 2];
    let last = closes[closes.len() - 1];
    if prev == 0.0 {
        return 0.0;
    }
    ((last - prev) / prev) * 100.0
}

/// One AnalyticalReport for a leader/follower pair; extracted so it can be
/// tested without network access.
fn build_pair_report(
    leader_label: &str,
    leader_data: &[models::MarketData],
    leader_closes: &[f64],
    follower_label: &str,
    follower_data: &[models::MarketData],
    timestamp: &str,
) -> models::AnalyticalReport {
    // Dates come from the candles, so the follower is needed whole, not just its closes.
    let leader_returns = dated_returns(leader_data);
    let follower_returns = dated_returns(follower_data);
    let correlation = align_and_correlate_lagged(&leader_returns, &follower_returns, DEFAULT_LAG);
    let overlap = overlapping_observations(&leader_returns, &follower_returns);
    // Volatility comes from the leader, not the follower; mixing these caused a bug.
    let volatility = analysis_engine::calculate_volatility(leader_data);
    // RSI/MACD also from the leader; the follower is only a reference point.
    let technicals = analysis_engine::calculate_technicals(leader_data);
    let latest_close = leader_closes.last().copied().unwrap_or(0.0);
    let daily_change = daily_change_pct(leader_closes);

    models::AnalyticalReport {
        symbol: format!("{}->{}", leader_label, follower_label),
        correlation,
        overlapping_observations: overlap,
        volatility,
        technicals,
        latest_close,
        daily_change_pct: daily_change,
        timestamp: timestamp.to_string(),
    }
}

pub(crate) async fn get_cross_market_analysis_inner() -> Result<Vec<models::AnalyticalReport>, CommandError> {
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
        for (follower_label, follower_data, _follower_closes) in &markets {
            if leader_label == follower_label {
                continue;
            }
            reports.push(build_pair_report(
                leader_label,
                leader_data,
                leader_closes,
                follower_label,
                follower_data,
                &timestamp,
            ));
        }
    }

    Ok(reports)
}

pub(crate) fn numeric_context_for_equity(instrument: &str, reports: &[models::AnalyticalReport]) -> String {
    let leader_prefix = format!("{}->", instrument);
    reports
        .iter()
        .filter(|r| r.symbol.starts_with(&leader_prefix))
        .map(|r| {
            let own_readings = format!(
                "zmienność={:.4}, RSI(14)={:.2}, MACD={:.4} (sygnał={:.4})",
                r.volatility, r.technicals.rsi, r.technicals.macd_line, r.technicals.macd_signal
            );
            match r.correlation {
                Some(value) => format!(
                    "- {}: korelacja={:.4} (wspólne sesje: {}), {}",
                    r.symbol, value, r.overlapping_observations, own_readings
                ),
                // An unmeasured correlation is left out rather than described. Any
                // wording here - "brak danych", "n/d", 0.0000 - is a fact the model
                // will comment on as confidently as a real reading; absence is the
                // only form it cannot misreport. The pair label drops with it,
                // because the arrow asserts the very relationship that was never
                // measured, and what remains belongs to the leader alone.
                None => format!("- {}: {}", instrument, own_readings),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod build_pair_report_tests {
    use super::*;

    fn candle(close: f64) -> models::MarketData {
        models::MarketData {
            symbol: "TEST".to_string(),
            time: "2026-01-01".to_string(),
            timestamp: 0,
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

        let report = build_pair_report(
            "NASDAQ",
            &leader_data,
            &leader_closes,
            "SP500",
            &follower_data,
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

        let report = build_pair_report("GOLD", &data, &closes, "SILVER", &data, "2026-01-01");
        assert_eq!(report.symbol, "GOLD->SILVER");
    }

    #[test]
    fn latest_close_is_leaders_last_candle() {
        let leader_data = vec![candle(100.0), candle(110.0), candle(105.0)];
        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();
        let follower_data = vec![candle(50.0), candle(51.0), candle(52.0)];

        let report = build_pair_report(
            "NASDAQ", &leader_data, &leader_closes, "SP500", &follower_data, "2026-01-01",
        );

        assert_eq!(report.latest_close, 105.0);
    }

    #[test]
    fn daily_change_pct_computed_from_leaders_last_two_candles() {
        let leader_data = vec![candle(100.0), candle(110.0)];
        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();
        let follower_data = vec![candle(50.0), candle(50.0)];

        let report = build_pair_report(
            "NASDAQ", &leader_data, &leader_closes, "SP500", &follower_data, "2026-01-01",
        );

        assert!((report.daily_change_pct - 10.0).abs() < 1e-9);
    }

    #[test]
    fn daily_change_pct_is_zero_for_single_candle() {
        assert_eq!(daily_change_pct(&[100.0]), 0.0);
        assert_eq!(daily_change_pct(&[]), 0.0);
    }

    #[test]
    fn daily_change_pct_ignores_zero_previous_close() {
        assert_eq!(daily_change_pct(&[0.0, 100.0]), 0.0);
    }

    fn dated_candle(day_offset: i64, close: f64) -> models::MarketData {
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

    #[test]
    fn unmeasured_correlation_still_yields_leader_readings() {
        let leader_data: Vec<models::MarketData> = (0..5)
            .map(|d| dated_candle(d, 100.0 + d as f64 * 3.0))
            .collect();
        let follower_data: Vec<models::MarketData> = (0..5)
            .map(|d| dated_candle(d, 50.0 - d as f64))
            .collect();
        let leader_closes: Vec<f64> = leader_data.iter().map(|d| d.close).collect();

        let report = build_pair_report(
            "NASDAQ", &leader_data, &leader_closes, "SP500", &follower_data, "2026-01-01",
        );

        // Five candles give four returns; far below the indicator threshold.
        assert_eq!(report.correlation, None);
        assert_eq!(report.overlapping_observations, 4);

        // Volatility and technicals do not depend on the join, so they survive.
        assert_eq!(report.volatility, analysis_engine::calculate_volatility(&leader_data));
        let expected = analysis_engine::calculate_technicals(&leader_data);
        assert_eq!(report.technicals.rsi, expected.rsi);
        assert_eq!(report.technicals.macd_line, expected.macd_line);
        assert_eq!(report.latest_close, 112.0);
    }

    fn report_with(correlation: Option<f64>, overlap: usize) -> models::AnalyticalReport {
        models::AnalyticalReport {
            symbol: "NASDAQ->SP500".to_string(),
            correlation,
            overlapping_observations: overlap,
            volatility: 0.0123,
            technicals: models::TechnicalIndicators {
                rsi: 55.25,
                macd_line: 1.5,
                macd_signal: 0.5,
            },
            latest_close: 100.0,
            daily_change_pct: 1.0,
            timestamp: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn prompt_context_carries_coefficient_and_sample_size() {
        let context = numeric_context_for_equity("NASDAQ", &[report_with(Some(0.8231), 41)]);

        assert!(context.contains("korelacja=0.8231"), "otrzymano {context}");
        assert!(context.contains("41"), "otrzymano {context}");
        assert!(context.contains("NASDAQ->SP500"), "otrzymano {context}");
        assert!(context.contains("RSI(14)=55.25"), "otrzymano {context}");
    }

    #[test]
    fn prompt_context_says_nothing_at_all_about_an_unmeasured_correlation() {
        let context = numeric_context_for_equity("NASDAQ", &[report_with(None, 4)]);

        let lowered = context.to_lowercase();
        assert!(!lowered.contains("korelacj"), "otrzymano {context}");
        assert!(!lowered.contains("correlation"), "otrzymano {context}");
        assert!(!context.contains("0.0000"), "otrzymano {context}");
        assert!(!context.contains("n/d"), "otrzymano {context}");
        // The arrow claims a leader/follower relationship that was never measured.
        assert!(!context.contains("->"), "otrzymano {context}");

        // What is left is the leader's own readings, unchanged.
        assert!(context.contains("- NASDAQ:"), "otrzymano {context}");
        assert!(context.contains("RSI(14)=55.25"), "otrzymano {context}");
    }
}

#[cfg(test)]
mod dated_series_tests {
    use super::*;
    use analysis_engine::MIN_CLOSES_FOR_INDICATORS as MIN;
    use time::{macros::date, Duration};

    fn day(offset: i64) -> Date {
        date!(2026 - 01 - 05) + Duration::days(offset)
    }

    fn candle_on(offset: i64, close: f64) -> models::MarketData {
        let date = day(offset);
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

    /// Non-periodic and non-constant, so a shifted copy never correlates at exactly 1.0.
    fn wave(i: i64) -> f64 {
        (i as f64) * 0.001 + ((i % 5) as f64) * 0.01
    }

    fn series(offsets: impl IntoIterator<Item = i64>, value: impl Fn(i64) -> f64) -> DatedSeries {
        offsets.into_iter().map(|d| (day(d), value(d))).collect()
    }

    #[test]
    fn correlation_uses_only_the_dates_both_calendars_share() {
        // The follower is closed on two sessions the leader trades.
        let leader = series(0..50, |d| if d == 3 || d == 7 { 5.0 } else { wave(d) });
        let follower = series((0..50).filter(|d| *d != 3 && *d != 7), wave);

        assert_eq!(overlapping_observations(&leader, &follower), 48);

        let correlation =
            align_and_correlate_lagged(&leader, &follower, 0).expect("48 wspólnych dni");
        // The leader-only outliers never enter the calculation, so the shared days match exactly.
        assert!((correlation - 1.0).abs() < 1e-9, "otrzymano {correlation}");
    }

    #[test]
    fn disjoint_calendars_report_no_measurement_rather_than_zero() {
        let leader = series(0..50, wave);
        let follower = series(200..250, wave);

        assert_eq!(overlapping_observations(&leader, &follower), 0);
        assert_eq!(align_and_correlate_lagged(&leader, &follower, 0), None);
    }

    #[test]
    fn overlap_exactly_at_the_threshold_is_measured() {
        let at_threshold = series(0..MIN as i64, wave);
        assert_eq!(overlapping_observations(&at_threshold, &at_threshold), MIN);
        assert!(align_and_correlate_lagged(&at_threshold, &at_threshold, 0).is_some());
    }

    #[test]
    fn overlap_one_below_the_threshold_is_not_measured() {
        let below = series(0..MIN as i64 - 1, wave);
        assert_eq!(overlapping_observations(&below, &below), MIN - 1);
        assert_eq!(align_and_correlate_lagged(&below, &below, 0), None);
    }

    #[test]
    fn lag_consumes_one_observation_against_the_threshold() {
        let at_threshold = series(0..MIN as i64, wave);
        assert_eq!(align_and_correlate_lagged(&at_threshold, &at_threshold, 1), None);
    }

    #[test]
    fn zero_close_drops_one_observation_without_shifting_the_rest() {
        let candles = vec![
            candle_on(0, 100.0),
            candle_on(1, 110.0),
            candle_on(2, 0.0),
            candle_on(3, 120.0),
            candle_on(4, 132.0),
        ];

        let returns = dated_returns(&candles);

        // Day 3 has no measurable return (previous close is zero); day 4 keeps its own.
        let dates: Vec<Date> = returns.iter().map(|(d, _)| *d).collect();
        assert_eq!(dates, vec![day(1), day(2), day(4)]);
        assert!((returns[2].1 - 0.1).abs() < 1e-9);
    }

    #[test]
    fn duplicate_date_keeps_the_later_occurrence() {
        let candles = vec![
            candle_on(0, 100.0),
            candle_on(1, 110.0),
            candle_on(1, 105.0),
        ];

        let returns = dated_returns(&candles);

        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].0, day(1));
        assert!((returns[0].1 - (105.0 - 110.0) / 110.0).abs() < 1e-9);
    }

    #[test]
    fn lag_shifts_by_a_shared_session_not_by_array_index() {
        // The follower is closed on two sessions, so its array indices and the
        // leader's drift apart by two.
        let shared: Vec<i64> = (0..45).filter(|d| *d != 5 && *d != 11).collect();

        let mut leader: DatedSeries = shared
            .iter()
            .enumerate()
            .map(|(i, d)| (day(*d), wave(i as i64)))
            .collect();
        leader.push((day(5), 42.0));
        leader.push((day(11), -42.0));
        leader.sort_by_key(|(d, _)| *d);

        // Follower on shared[k] repeats what the leader printed on shared[k - 1],
        // so a lag of one shared session lines the two up perfectly.
        let follower: DatedSeries = shared
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, d)| (day(*d), wave(i as i64 - 1)))
            .collect();

        let correlation =
            align_and_correlate_lagged(&leader, &follower, 1).expect("42 wspólne dni");
        assert!((correlation - 1.0).abs() < 1e-9, "otrzymano {correlation}");

        // Without the lag the same pair is merely well-correlated, not identical.
        let unlagged = align_and_correlate_lagged(&leader, &follower, 0).expect("42 wspólne dni");
        assert!((unlagged - 1.0).abs() > 1e-6, "otrzymano {unlagged}");
    }

    #[test]
    fn join_drops_gaps_instead_of_forward_filling_them() {
        let a = series(0..10, |d| d as f64);
        let b = series((0..10).filter(|d| *d != 4), |d| d as f64 * 2.0);

        let (values_a, values_b) = join_on_date(&a, &b);

        assert_eq!(values_a.len(), 9);
        assert_eq!(values_b.len(), 9);
        // No synthetic 0% return stands in for the missing day.
        assert_eq!(values_a, vec![0.0, 1.0, 2.0, 3.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        assert_eq!(values_b, vec![0.0, 2.0, 4.0, 6.0, 10.0, 12.0, 14.0, 16.0, 18.0]);
    }
}
