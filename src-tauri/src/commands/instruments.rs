//! Thin lookup layer over the crate-level catalogue. Kept as a separate module
//! because the command layer asks narrow questions ("is this supported?",
//! "which Yahoo symbol?") and should not carry catalogue rows around.

use crate::catalog::{self, InstrumentClass};

pub(crate) fn is_supported(instrument: &str) -> bool {
    catalog::find(instrument).is_some()
}

/// Metals take a different numeric path (GSR, Au-Ag correlation) than equities.
pub(crate) fn is_metal(instrument: &str) -> bool {
    catalog::find(instrument).is_some_and(|entry| entry.class == InstrumentClass::Metal)
}

/// `None` for unknown ids. Previously an unknown id resolved to "^GSPC", which
/// let stored tactics be verified against S&P 500 prices instead of their own
/// instrument; callers must now handle the miss explicitly.
pub(crate) fn yahoo_symbol_for(instrument: &str) -> Option<&'static str> {
    catalog::find(instrument).map(|entry| entry.yahoo_symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_original_four_instruments() {
        assert_eq!(yahoo_symbol_for("NASDAQ"), Some("^IXIC"));
        assert_eq!(yahoo_symbol_for("SP500"), Some("^GSPC"));
        assert_eq!(yahoo_symbol_for("GOLD"), Some("GC=F"));
        assert_eq!(yahoo_symbol_for("SILVER"), Some("SI=F"));
    }

    #[test]
    fn unknown_instrument_has_no_symbol() {
        assert_eq!(yahoo_symbol_for("BITCOIN"), None);
    }

    #[test]
    fn is_metal_matches_only_metals() {
        assert!(is_metal("GOLD"));
        assert!(is_metal("SILVER"));
        assert!(!is_metal("NASDAQ"));
        assert!(!is_metal("SP500"));
        assert!(!is_metal("GLD"), "ETF na złoto nie jest metalem w sensie GSR");
    }

    #[test]
    fn is_supported_rejects_unknown() {
        assert!(is_supported("NASDAQ"));
        assert!(!is_supported("BITCOIN"));
    }
}
