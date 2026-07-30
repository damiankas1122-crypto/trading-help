//! Pine Script for equity correlation (NASDAQ<->SP500) and the Gold/Silver
//! Ratio - both hand-written templates, no AI. `find_strongest_equity_pair`
//! decides which equity pair reaches `generate_correlation_pine_script` (see
//! commands/market_context.rs). Depends on no other ai_engine submodule; the
//! TradingView tickers come from the catalogue entries passed in by the caller.

use crate::catalog::Instrument;
use crate::models::AnalyticalReport;

/// "Strongest" is measured only across the equity reports passed in by the
/// caller (today NASDAQ<->SP500); metals have their own GSR script and never
/// enter this comparison.
///
/// Returns `None` for empty input and, since correlation became optional, when
/// no report carries a usable figure. That case has to end with no correlation
/// script at all: a Pine indicator plotting `ta.correlation` for a pair whose
/// correlation was never measured is the same error as printing the number.
///
/// Two traps live in this comparison, both avoided deliberately:
///
/// - `Option<f64>` implements `PartialOrd` with `None < Some(_)`, so comparing
///   the options directly compiles, ranks an unmeasured pair as merely the
///   weakest, and - if `.abs()` slips out in the rewrite - lets +0.1 beat -0.9.
///   Unmeasured reports are filtered out before any comparison rather than
///   ordered against measured ones, and `.abs()` is applied inside `Some`.
/// - `Some(NaN)` is a separate case from `None`: measured, but not comparable.
///   It is dropped here too, so `partial_cmp` below always has a total order and
///   no ordering fallback can hand it the win.
pub fn find_strongest_equity_pair(reports: &[AnalyticalReport]) -> Option<&AnalyticalReport> {
    reports
        .iter()
        .filter_map(|report| {
            report
                .correlation
                .filter(|value| value.is_finite())
                .map(|value| (report, value.abs()))
        })
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(report, _)| report)
}

/// Takes resolved catalogue entries rather than a "LEADER->FOLLOWER" string:
/// parsing it here meant an unparseable pair silently rendered a NASDAQ/SP500
/// script. Resolution now happens in the command layer, which can report the
/// failure instead of substituting another market.
pub fn generate_correlation_pine_script(leader: &Instrument, follower: &Instrument) -> String {
    let (leader_label, follower_label) = (leader.label, follower.label);
    let (leader_ticker, follower_ticker) = (leader.tv_ticker, follower.tv_ticker);

    format!(
        r#"//@version=6
indicator("Trading Help: {leader_label}/{follower_label} Correlation", overlay=false)

lengthInput = input.int(20, title="Okno korelacji (świece)")
lagInput = input.int(1, title="Przesunięcie (lag, świece)")

leaderClose = request.security("{leader_ticker}", timeframe.period, close)
followerClose = request.security("{follower_ticker}", timeframe.period, close)
leaderShifted = leaderClose[lagInput]
correlation = ta.correlation(leaderShifted, followerClose, lengthInput)

plot(correlation, title="Korelacja {leader_label}->{follower_label}", color=color.aqua)
hline(0, "Zero", color=color.gray)
hline(0.5, "+0.5", color=color.green)
hline(-0.5, "-0.5", color=color.red)
"#,
        leader_label = leader_label,
        follower_label = follower_label,
        leader_ticker = leader_ticker,
        follower_ticker = follower_ticker,
    )
}

/// Fixed explanation of the correlation script.
pub fn explain_correlation_script(leader: &Instrument, follower: &Instrument) -> String {
    let (leader_label, follower_label) = (leader.label, follower.label);

    format!(
        "Ten wskaźnik pokazuje, jak silnie {leader_label} 'przewiduje' ruch {follower_label} \
         z jednodniowym wyprzedzeniem. Linia korelacji porusza się w zakresie od -1 do +1: \
         wartości bliskie +1 oznaczają, że wzrost {leader_label} wczoraj zwykle poprzedza wzrost \
         {follower_label} dzisiaj; wartości bliskie -1 oznaczają zależność odwrotną; wartości \
         bliskie 0 oznaczają brak przewidywalnego związku.\n\n\
         Parametr 'Okno korelacji' (domyślnie 20 świec) to liczba dni branych pod uwagę przy \
         każdym przeliczeniu - mniejsza wartość daje bardziej czułą, ale bardziej 'szarpaną' \
         linię; większa wartość wygładza wykres, ale wolniej reaguje na zmiany.\n\n\
         Parametr 'Przesunięcie (lag)' określa, o ile sesji do przodu sprawdzamy wpływ - domyślnie \
         1, zgodnie z analizą w aplikacji. Oba parametry możesz swobodnie zmieniać w ustawieniach \
         wskaźnika w TradingView (ikona koła zębatego przy nazwie wskaźnika).",
        leader_label = leader_label,
        follower_label = follower_label,
    )
}

pub fn generate_gsr_pine_script() -> String {
    r#"//@version=6
indicator("Trading Help: Gold/Silver Ratio (GSR)", overlay=false)

maLengthInput = input.int(50, title="Okres średniej kroczącej")
highBandInput = input.float(80.0, title="Górne pasmo GSR")
lowBandInput = input.float(50.0, title="Dolne pasmo GSR")

gsr = request.security("TVC:GOLDSILVER", timeframe.period, close)
gsrMa = ta.sma(gsr, maLengthInput)

plot(gsr, title="GSR", color=color.yellow, linewidth=2)
plot(gsrMa, title="Średnia krocząca GSR", color=color.blue)
hline(highBandInput, "GSR wysoki", color=color.red)
hline(lowBandInput, "GSR niski", color=color.green)
"#
    .to_string()
}

/// Fixed explanation of the GSR script.
pub fn explain_gsr_script() -> String {
    "Ten wskaźnik pokazuje relację Gold/Silver Ratio (GSR) - ile uncji srebra kosztuje jedna \
     uncja złota - bezpośrednio z wbudowanego w TradingView indeksu GOLDSILVER, więc nie musi \
     nic samodzielnie przeliczać.\n\n\
     Żółta linia to bieżąca wartość GSR, niebieska to jej średnia krocząca (domyślnie z 50 \
     świec) pokazująca długoterminowy trend bez dziennego 'szumu'.\n\n\
     Czerwona pozioma linia (domyślnie 80) oznacza historycznie wysoki poziom GSR - zwykle \
     interpretowany jako srebro relatywnie tanie względem złota. Zielona pozioma linia \
     (domyślnie 50) oznacza historycznie niski poziom - srebro relatywnie drogie względem \
     złota. Oba progi możesz dowolnie zmienić w ustawieniach wskaźnika, żeby dopasować je do \
     własnej analizy historycznej - to tylko orientacyjne wartości domyślne, nie sztywna reguła."
        .to_string()
}

#[cfg(test)]
mod find_strongest_equity_pair_tests {
    use super::*;

    fn report(symbol: &str, correlation: Option<f64>) -> AnalyticalReport {
        AnalyticalReport {
            symbol: symbol.to_string(),
            correlation,
            overlapping_observations: correlation.map_or(0, |_| 40),
            volatility: 0.0,
            technicals: crate::models::TechnicalIndicators { rsi: 50.0, macd_line: 0.0, macd_signal: 0.0 },
            latest_close: 0.0,
            daily_change_pct: 0.0,
            timestamp: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn picks_report_with_highest_absolute_correlation() {
        let reports = vec![
            report("NASDAQ->SP500", Some(0.2)),
            report("SP500->NASDAQ", Some(0.85)),
            report("GOLD->SILVER", Some(-0.4)),
        ];

        let strongest = find_strongest_equity_pair(&reports).expect("expected a report");
        assert_eq!(strongest.symbol, "SP500->NASDAQ");
    }

    #[test]
    fn negative_correlation_with_larger_magnitude_beats_smaller_positive() {
        let reports = vec![
            report("NASDAQ->SP500", Some(0.3)),
            report("GOLD->SILVER", Some(-0.9)),
        ];

        let strongest = find_strongest_equity_pair(&reports).expect("expected a report");
        assert_eq!(strongest.symbol, "GOLD->SILVER");
    }

    #[test]
    fn strong_negative_still_beats_weak_positive_after_the_option_rewrite() {
        // Comparing the options directly would compile and drop `.abs()`, letting
        // +0.1 win.
        let reports = vec![
            report("NASDAQ->SP500", Some(0.1)),
            report("SP500->NASDAQ", Some(-0.9)),
        ];

        let strongest = find_strongest_equity_pair(&reports).expect("expected a report");
        assert_eq!(strongest.symbol, "SP500->NASDAQ");
    }

    #[test]
    fn unmeasured_pair_is_excluded_rather_than_ranked_last() {
        // `None < Some(_)` would place the unmeasured pair at the bottom, which is
        // right here by accident; the point is that it is never a candidate.
        let reports = vec![
            report("NASDAQ->SP500", None),
            report("SP500->NASDAQ", Some(0.4)),
        ];

        let strongest = find_strongest_equity_pair(&reports).expect("expected a report");
        assert_eq!(strongest.symbol, "SP500->NASDAQ");
        assert_eq!(strongest.correlation, Some(0.4));
    }

    #[test]
    fn no_measured_correlation_means_no_pair_to_script() {
        let reports = vec![
            report("NASDAQ->SP500", None),
            report("SP500->NASDAQ", None),
        ];

        assert!(find_strongest_equity_pair(&reports).is_none());
    }

    #[test]
    fn does_not_panic_when_correlation_is_nan() {
        let reports = vec![
            report("NASDAQ->SP500", Some(f64::NAN)),
            report("SP500->NASDAQ", Some(0.5)),
        ];

        let result = find_strongest_equity_pair(&reports);
        assert!(result.is_some());
    }

    #[test]
    fn nan_never_wins_against_a_measured_correlation() {
        // `Some(NaN)` is measured but not comparable; it must not survive the
        // ordering fallback, whatever position it holds in the list.
        let leading_nan = vec![
            report("NASDAQ->SP500", Some(f64::NAN)),
            report("SP500->NASDAQ", Some(0.5)),
        ];
        let trailing_nan = vec![
            report("SP500->NASDAQ", Some(0.5)),
            report("NASDAQ->SP500", Some(f64::NAN)),
        ];

        for reports in [leading_nan, trailing_nan] {
            let strongest = find_strongest_equity_pair(&reports).expect("expected a report");
            assert_eq!(strongest.symbol, "SP500->NASDAQ");
        }
    }

    #[test]
    fn only_nan_correlations_leave_nothing_to_pick() {
        let reports = vec![report("NASDAQ->SP500", Some(f64::NAN))];
        assert!(find_strongest_equity_pair(&reports).is_none());
    }

    #[test]
    fn returns_none_for_empty_reports() {
        let reports: Vec<AnalyticalReport> = vec![];
        assert!(find_strongest_equity_pair(&reports).is_none());
    }
}
