//! Pine Script dla korelacji equity (NASDAQ<->SP500) i Gold/Silver Ratio -
//! oba w 100% ręcznie napisane szablony, zero AI. `find_strongest_pair`
//! wybiera, która para equity trafia do `generate_correlation_pine_script`
//! (patrz commands/briefing.rs). Nie zależy od żadnego innego submodułu
//! ai_engine poza `label_to_tv_ticker` z `mod.rs`.

use crate::models::AnalyticalReport;
use super::label_to_tv_ticker;

pub fn find_strongest_pair(reports: &[AnalyticalReport]) -> Option<&AnalyticalReport> {
    reports.iter().max_by(|a, b| {
        a.correlation
            .abs()
            .partial_cmp(&b.correlation.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub fn generate_correlation_pine_script(equity_pair_symbol: &str) -> String {
    let parts: Vec<&str> = equity_pair_symbol.split("->").collect();
    let (leader_label, follower_label) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("NASDAQ", "SP500")
    };

    let leader_ticker = label_to_tv_ticker(leader_label);
    let follower_ticker = label_to_tv_ticker(follower_label);

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

/// wyjaśnienie skryptu korelacji, na sztywno
pub fn explain_correlation_script(equity_pair_symbol: &str) -> String {
    let parts: Vec<&str> = equity_pair_symbol.split("->").collect();
    let (leader_label, follower_label) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("NASDAQ", "SP500")
    };

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

/// wyjaśnienie skryptu GSR, na sztywno
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
mod find_strongest_pair_tests {
    use super::*;

    fn report(symbol: &str, correlation: f64) -> AnalyticalReport {
        AnalyticalReport {
            symbol: symbol.to_string(),
            correlation,
            volatility: 0.0,
            technicals: crate::models::TechnicalIndicators { rsi: 50.0, macd_line: 0.0, macd_signal: 0.0 },
            timestamp: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn picks_report_with_highest_absolute_correlation() {
        let reports = vec![
            report("NASDAQ->SP500", 0.2),
            report("SP500->NASDAQ", 0.85),
            report("GOLD->SILVER", -0.4),
        ];

        let strongest = find_strongest_pair(&reports).expect("powinien znaleźć raport");
        assert_eq!(strongest.symbol, "SP500->NASDAQ");
    }

    #[test]
    fn negative_correlation_with_larger_magnitude_beats_smaller_positive() {
        let reports = vec![
            report("NASDAQ->SP500", 0.3),
            report("GOLD->SILVER", -0.9),
        ];

        let strongest = find_strongest_pair(&reports).expect("powinien znaleźć raport");
        assert_eq!(strongest.symbol, "GOLD->SILVER");
    }

    #[test]
    fn does_not_panic_when_correlation_is_nan() {
        let reports = vec![
            report("NASDAQ->SP500", f64::NAN),
            report("SP500->NASDAQ", 0.5),
        ];

        let result = find_strongest_pair(&reports);
        assert!(result.is_some());
    }

    #[test]
    fn returns_none_for_empty_reports() {
        let reports: Vec<AnalyticalReport> = vec![];
        assert!(find_strongest_pair(&reports).is_none());
    }
}
