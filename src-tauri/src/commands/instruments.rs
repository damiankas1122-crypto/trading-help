//! Supported instruments and their Yahoo Finance symbols - the single source of
//! truth for the command layer. The same list and mapping previously lived
//! separately in `instrument_briefing.rs` and `tactics.rs`.

pub(crate) const VALID_INSTRUMENTS: [&str; 4] = ["NASDAQ", "SP500", "GOLD", "SILVER"];

pub(crate) fn is_supported(instrument: &str) -> bool {
    VALID_INSTRUMENTS.contains(&instrument)
}

/// Metals take a different numeric path (GSR, Au-Ag correlation) than equities.
pub(crate) fn is_metal(instrument: &str) -> bool {
    instrument == "GOLD" || instrument == "SILVER"
}

pub(crate) fn yahoo_symbol_for(instrument: &str) -> &'static str {
    match instrument {
        "NASDAQ" => "^IXIC",
        "SP500" => "^GSPC",
        "GOLD" => "GC=F",
        "SILVER" => "SI=F",
        _ => "^GSPC",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_valid_instrument_has_a_dedicated_yahoo_symbol() {
        // The "^GSPC" fallback exists only for unknown labels; no supported
        // instrument may reach it by accident.
        for instrument in VALID_INSTRUMENTS {
            let symbol = yahoo_symbol_for(instrument);
            if instrument != "SP500" {
                assert_ne!(symbol, "^GSPC", "{instrument} wpada na fallback");
            }
        }
    }

    #[test]
    fn is_metal_matches_only_metals() {
        assert!(is_metal("GOLD"));
        assert!(is_metal("SILVER"));
        assert!(!is_metal("NASDAQ"));
        assert!(!is_metal("SP500"));
    }

    #[test]
    fn is_supported_rejects_unknown() {
        assert!(is_supported("NASDAQ"));
        assert!(!is_supported("BITCOIN"));
    }
}
