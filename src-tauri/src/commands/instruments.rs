//! Wspierane instrumenty i ich symbole Yahoo Finance - jedno źródło prawdy
//! dla warstwy komend (wcześniej ta sama lista i to samo mapowanie żyły
//! osobno w `instrument_briefing.rs` i `tactics.rs`).

pub(crate) const VALID_INSTRUMENTS: [&str; 4] = ["NASDAQ", "SP500", "GOLD", "SILVER"];

pub(crate) fn is_supported(instrument: &str) -> bool {
    VALID_INSTRUMENTS.contains(&instrument)
}

/// metale mają własną ścieżkę liczbową (GSR/korelacja Au-Ag) niż equity
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
        // fallback "^GSPC" jest tylko dla nieznanych - żaden wspierany
        // instrument nie może na niego wpaść przez pomyłkę
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
