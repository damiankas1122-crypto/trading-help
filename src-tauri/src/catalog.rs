//! Instrument catalogue - the single source of truth for every instrument the
//! app supports. Everything that used to be a separate `match` on a string
//! (Yahoo symbol, TradingView ticker, metal flag, supported list) is one row
//! here, so adding an instrument means adding one entry and nothing else.
//!
//! Lives at crate root rather than under `commands` because it is domain data,
//! not a command-layer concern: `ai_engine` needs `tv_ticker` and `news_engine`
//! will need `keywords`, and neither may depend on `commands`.
//!
//! Lookups never fall back to a default instrument: an unknown id is an error
//! for the caller to handle, not something to silently resolve to S&P 500.

// Still without a consumer: `description`, `benchmark_id`, `news_symbol` and
// `by_class`, all of which the instrument panel will use. They are part of the
// intended row shape, not leftovers.
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentClass {
    Metal,
    Index,
    Etf,
    Macro,
    Sector,
    Stock,
}

pub struct Instrument {
    /// Identifier used in the command API and persisted in `tactics.json`.
    pub id: &'static str,
    pub label: &'static str,
    /// User-facing, hence Polish.
    pub description: &'static str,
    pub yahoo_symbol: &'static str,
    pub tv_ticker: &'static str,
    pub class: InstrumentClass,
    /// Catalogue id this instrument is correlated against.
    pub benchmark_id: Option<&'static str>,
    /// Catalogue id whose exchange ticker carries per-instrument news.
    /// The 2026-07-28 data spike showed Finnhub `/company-news` returns nothing
    /// for indices and futures, so those point at a listed proxy (GOLD -> GLD)
    /// or at `None` when no honest proxy exists. `None` means "no news source",
    /// which the UI must show as such - never as zero news.
    pub news_symbol: Option<&'static str>,
    /// Whole-word matched against RSS headlines in `news_engine`. English only:
    /// the feeds are English and matching has no stemming, so Polish forms would
    /// never hit. Empty means no feed covers this instrument - see
    /// `news_engine::NewsSource`.
    pub keywords: &'static [&'static str],
}

/// Rows marked as unverified were not covered by the 2026-07-28 data spike;
/// their Yahoo symbols and news availability are assumed, not measured.
static CATALOG: &[Instrument] = &[
    // --- Metals -----------------------------------------------------------
    Instrument {
        id: "GOLD",
        label: "Złoto",
        description: "Kontrakt terminowy na złoto notowany na COMEX.",
        yahoo_symbol: "GC=F",
        tv_ticker: "TVC:GOLD",
        class: InstrumentClass::Metal,
        benchmark_id: Some("SILVER"),
        news_symbol: Some("GLD"),
        keywords: &["gold", "bullion", "safe haven", "precious metals"],
    },
    Instrument {
        id: "SILVER",
        label: "Srebro",
        description: "Kontrakt terminowy na srebro notowany na COMEX.",
        yahoo_symbol: "SI=F",
        tv_ticker: "TVC:SILVER",
        class: InstrumentClass::Metal,
        benchmark_id: Some("GOLD"),
        news_symbol: Some("SLV"),
        keywords: &["silver", "precious metals"],
    },
    // --- ETFs -------------------------------------------------------------
    Instrument {
        id: "GLD",
        label: "SPDR Gold Shares",
        description: "Największy ETF na złoto, zabezpieczony fizycznym kruszcem.",
        yahoo_symbol: "GLD",
        tv_ticker: "AMEX:GLD",
        class: InstrumentClass::Etf,
        benchmark_id: Some("GOLD"),
        news_symbol: Some("GLD"),
        keywords: &["gold", "bullion"],
    },
    Instrument {
        id: "SLV",
        label: "iShares Silver Trust",
        description: "Największy ETF na srebro, zabezpieczony fizycznym kruszcem.",
        yahoo_symbol: "SLV",
        tv_ticker: "AMEX:SLV",
        class: InstrumentClass::Etf,
        benchmark_id: Some("SILVER"),
        news_symbol: Some("SLV"),
        keywords: &["silver"],
    },
    Instrument {
        // Not covered by the data spike - Yahoo/Finnhub behaviour assumed.
        id: "IAU",
        label: "iShares Gold Trust",
        description: "Tańszy kosztowo odpowiednik GLD, również oparty na fizycznym złocie.",
        yahoo_symbol: "IAU",
        tv_ticker: "AMEX:IAU",
        class: InstrumentClass::Etf,
        benchmark_id: Some("GOLD"),
        news_symbol: Some("IAU"),
        keywords: &["gold", "bullion"],
    },
    Instrument {
        id: "GDX",
        label: "VanEck Gold Miners",
        description: "ETF spółek wydobywających złoto — lewarowana ekspozycja na cenę kruszcu.",
        yahoo_symbol: "GDX",
        tv_ticker: "AMEX:GDX",
        class: InstrumentClass::Etf,
        benchmark_id: Some("GLD"),
        news_symbol: Some("GDX"),
        // The only ticker kept as a keyword: "GDX" does appear in gold-mining
        // headlines, and this instrument has the thinnest news coverage of the group.
        keywords: &["gdx", "gold miners", "mining stocks"],
    },
    Instrument {
        id: "SPY",
        label: "SPDR S&P 500",
        description: "Najpłynniejszy ETF odwzorowujący indeks S&P 500.",
        yahoo_symbol: "SPY",
        tv_ticker: "AMEX:SPY",
        class: InstrumentClass::Etf,
        benchmark_id: Some("SP500"),
        news_symbol: Some("SPY"),
        // Never "spy": it is an ordinary English word, and word boundaries do not
        // help when the keyword itself is the false positive.
        keywords: &["s&p 500", "s&p500", "stock market"],
    },
    Instrument {
        id: "QQQ",
        label: "Invesco QQQ",
        description: "ETF odwzorowujący indeks Nasdaq 100, zdominowany przez big tech.",
        yahoo_symbol: "QQQ",
        tv_ticker: "NASDAQ:QQQ",
        class: InstrumentClass::Etf,
        benchmark_id: Some("NASDAQ"),
        news_symbol: Some("QQQ"),
        keywords: &["nasdaq 100", "nasdaq", "big tech"],
    },
    Instrument {
        // Not covered by the data spike - Yahoo/Finnhub behaviour assumed.
        id: "DIA",
        label: "SPDR Dow Jones",
        description: "ETF odwzorowujący indeks Dow Jones Industrial Average.",
        yahoo_symbol: "DIA",
        tv_ticker: "AMEX:DIA",
        class: InstrumentClass::Etf,
        benchmark_id: Some("DOWJONES"),
        news_symbol: Some("DIA"),
        // Never bare "dow": it matches Dow Inc., a chemical company covered by the
        // same feeds.
        keywords: &["dow jones", "dow industrials"],
    },
    Instrument {
        id: "IWM",
        label: "iShares Russell 2000",
        description: "ETF na indeks małych spółek Russell 2000 — barometr apetytu na ryzyko.",
        yahoo_symbol: "IWM",
        tv_ticker: "AMEX:IWM",
        class: InstrumentClass::Etf,
        benchmark_id: Some("RUSSELL2000"),
        news_symbol: Some("IWM"),
        keywords: &["russell 2000", "small caps"],
    },
    // --- Indices ----------------------------------------------------------
    Instrument {
        id: "SP500",
        label: "S&P 500",
        description: "Indeks 500 największych spółek giełdy amerykańskiej.",
        yahoo_symbol: "^GSPC",
        tv_ticker: "SP:SPX",
        class: InstrumentClass::Index,
        benchmark_id: Some("NASDAQ"),
        news_symbol: Some("SPY"),
        // Deliberately broad: macro drivers move the index. This is also the
        // noisiest set in the catalogue.
        keywords: &[
            "s&p 500", "s&p500", "wall street", "stock market", "equities", "fed",
            "federal reserve", "inflation",
        ],
    },
    Instrument {
        id: "NASDAQ",
        label: "NASDAQ",
        description: "Indeks giełdy Nasdaq, zdominowany przez spółki technologiczne.",
        yahoo_symbol: "^IXIC",
        tv_ticker: "NASDAQ:IXIC",
        class: InstrumentClass::Index,
        benchmark_id: Some("SP500"),
        news_symbol: Some("QQQ"),
        keywords: &["nasdaq", "tech stocks", "technology sector", "big tech", "ai stocks"],
    },
    Instrument {
        // Not covered by the data spike - Yahoo/Finnhub behaviour assumed.
        id: "DOWJONES",
        label: "Dow Jones",
        description: "Indeks 30 największych spółek przemysłowych USA.",
        yahoo_symbol: "^DJI",
        tv_ticker: "DJ:DJI",
        class: InstrumentClass::Index,
        benchmark_id: Some("SP500"),
        news_symbol: Some("DIA"),
        keywords: &["dow jones", "dow industrials", "blue chips"],
    },
    Instrument {
        id: "RUSSELL2000",
        label: "Russell 2000",
        description: "Indeks małych spółek amerykańskich, wrażliwy na koszt pieniądza.",
        yahoo_symbol: "^RUT",
        tv_ticker: "TVC:RUT",
        class: InstrumentClass::Index,
        benchmark_id: Some("SP500"),
        news_symbol: Some("IWM"),
        // Never bare "russell": Russell Investments and personal names.
        keywords: &["russell 2000", "small caps", "small-cap stocks"],
    },
    Instrument {
        id: "VIX",
        label: "VIX",
        description: "Indeks zmienności S&P 500, potocznie „wskaźnik strachu”.",
        yahoo_symbol: "^VIX",
        tv_ticker: "TVC:VIX",
        class: InstrumentClass::Index,
        benchmark_id: Some("SP500"),
        // No listed proxy carries VIX-specific news.
        news_symbol: None,
        // No feed covers volatility as an instrument; empty means "no source".
        keywords: &[],
    },
    // --- Macro ------------------------------------------------------------
    Instrument {
        id: "DXY",
        label: "Dolar (DXY)",
        description: "Indeks siły dolara wobec koszyka głównych walut.",
        yahoo_symbol: "DX-Y.NYB",
        tv_ticker: "TVC:DXY",
        class: InstrumentClass::Macro,
        benchmark_id: Some("SP500"),
        news_symbol: None,
        keywords: &[],
    },
    Instrument {
        id: "US10Y",
        label: "Rentowność 10Y",
        description: "Rentowność 10-letnich obligacji skarbowych USA.",
        yahoo_symbol: "^TNX",
        tv_ticker: "TVC:US10Y",
        class: InstrumentClass::Macro,
        benchmark_id: Some("SP500"),
        news_symbol: None,
        keywords: &[],
    },
    Instrument {
        id: "OIL",
        label: "Ropa WTI",
        description: "Kontrakt terminowy na ropę naftową WTI.",
        yahoo_symbol: "CL=F",
        tv_ticker: "TVC:USOIL",
        class: InstrumentClass::Macro,
        benchmark_id: Some("SP500"),
        news_symbol: None,
        // No macro feed yet; bare "oil" would also match "oil painting" and the like.
        keywords: &[],
    },
    // --- Sectors ----------------------------------------------------------
    Instrument {
        id: "XLK",
        label: "Sektor technologiczny",
        description: "ETF sektora technologicznego z indeksu S&P 500.",
        yahoo_symbol: "XLK",
        tv_ticker: "AMEX:XLK",
        class: InstrumentClass::Sector,
        benchmark_id: Some("SPY"),
        news_symbol: Some("XLK"),
        keywords: &["tech stocks", "technology sector", "semiconductors"],
    },
    Instrument {
        id: "XLE",
        label: "Sektor energetyczny",
        description: "ETF sektora energetycznego z indeksu S&P 500.",
        yahoo_symbol: "XLE",
        tv_ticker: "AMEX:XLE",
        class: InstrumentClass::Sector,
        benchmark_id: Some("SPY"),
        news_symbol: Some("XLE"),
        // Never bare "oil": "oil painting", "oil spill", "olive oil".
        keywords: &["energy stocks", "energy sector", "oil majors"],
    },
];

/// Returns `None` for unknown ids - callers decide what an unsupported
/// instrument means, no default is substituted.
pub fn find(id: &str) -> Option<&'static Instrument> {
    CATALOG.iter().find(|instrument| instrument.id == id)
}

pub fn all() -> &'static [Instrument] {
    CATALOG
}

pub fn by_class(class: InstrumentClass) -> impl Iterator<Item = &'static Instrument> {
    CATALOG.iter().filter(move |instrument| instrument.class == class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for instrument in all() {
            assert!(seen.insert(instrument.id), "zduplikowane id: {}", instrument.id);
        }
    }

    #[test]
    fn benchmarks_point_at_existing_instruments() {
        for instrument in all() {
            if let Some(benchmark) = instrument.benchmark_id {
                assert!(
                    find(benchmark).is_some(),
                    "{} wskazuje na nieistniejący benchmark {benchmark}",
                    instrument.id
                );
                assert_ne!(benchmark, instrument.id, "{} jest własnym benchmarkiem", instrument.id);
            }
        }
    }

    #[test]
    fn news_proxies_point_at_existing_instruments() {
        for instrument in all() {
            if let Some(proxy) = instrument.news_symbol {
                assert!(
                    find(proxy).is_some(),
                    "{} wskazuje na nieistniejące proxy newsów {proxy}",
                    instrument.id
                );
            }
        }
    }

    #[test]
    fn news_proxies_are_listed_tickers() {
        // The spike showed Finnhub only answers for exchange-listed symbols;
        // a proxy pointing at an index or future would fail silently.
        for instrument in all() {
            let Some(proxy) = instrument.news_symbol else { continue };
            let target = find(proxy).expect("proxy istnieje");
            assert!(
                matches!(
                    target.class,
                    InstrumentClass::Etf | InstrumentClass::Sector | InstrumentClass::Stock
                ),
                "{} używa proxy {proxy}, które nie jest notowanym tickerem",
                instrument.id
            );
        }
    }

    #[test]
    fn find_returns_none_for_unknown_id() {
        assert!(find("BITCOIN").is_none());
        assert!(find("").is_none());
        assert!(find("sp500").is_none(), "id są wielkością liter znaczące");
    }

    #[test]
    fn by_class_filters() {
        let metals: Vec<_> = by_class(InstrumentClass::Metal).map(|i| i.id).collect();
        assert_eq!(metals, vec!["GOLD", "SILVER"]);
        assert!(by_class(InstrumentClass::Stock).next().is_none());
    }

    #[test]
    fn tv_tickers_are_exchange_qualified() {
        // A bare symbol resolves to whatever exchange TradingView guesses, which
        // can silently be a different instrument than intended.
        for instrument in all() {
            let ticker = instrument.tv_ticker;
            let mut parts = ticker.split(':');
            let exchange = parts.next().unwrap_or("");
            let symbol = parts.next().unwrap_or("");
            assert!(
                !exchange.is_empty() && !symbol.is_empty() && parts.next().is_none(),
                "{} ma tv_ticker '{ticker}' spoza formatu GIEŁDA:SYMBOL",
                instrument.id
            );
        }
    }

    #[test]
    fn every_instrument_has_a_description() {
        for instrument in all() {
            assert!(!instrument.description.is_empty(), "{} bez opisu", instrument.id);
        }
    }

    #[test]
    fn keyword_presence_matches_having_a_news_source() {
        // Empty keywords are the "no news source" marker, so they may not appear
        // on an instrument that does have one - that would read as "zero news".
        for instrument in all() {
            if instrument.news_symbol.is_some() {
                assert!(
                    !instrument.keywords.is_empty(),
                    "{} ma źródło newsów, ale nie ma słów kluczowych",
                    instrument.id
                );
            } else {
                assert!(
                    instrument.keywords.is_empty(),
                    "{} nie ma źródła newsów, więc słowa kluczowe są nieosiągalne",
                    instrument.id
                );
            }
        }
    }

    #[test]
    fn keywords_are_lowercase_ascii() {
        // Headlines are lowercased before matching, and the feeds are English;
        // an uppercase or Polish keyword would silently never match.
        for instrument in all() {
            for keyword in instrument.keywords {
                assert!(
                    keyword.chars().all(|c| !c.is_uppercase()) && keyword.is_ascii(),
                    "{}: słowo kluczowe '{keyword}' nigdy nie trafi",
                    instrument.id
                );
            }
        }
    }
}
