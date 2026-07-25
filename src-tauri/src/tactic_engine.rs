// src-tauri/src/tactic_engine.rs
use crate::models::{MarketData, TacticTrackRecord, TrackedTactic};

pub const HOUR_24_SECONDS: i64 = 24 * 60 * 60;
pub const DAYS_7_SECONDS: i64 = 7 * HOUR_24_SECONDS;


pub fn evaluate_outcome(tactic: &TrackedTactic, price_history: &[MarketData], until: i64) -> Option<&'static str> {
    if tactic.scenario == "neutral" {
        return None;
    }

    let target_price = tactic.reference_price * (1.0 + tactic.target_pct / 100.0);
    let stop_price = tactic.reference_price * (1.0 + tactic.stop_loss_pct / 100.0);

    let mut relevant: Vec<&MarketData> = price_history
        .iter()
        .filter(|d| d.timestamp > tactic.generated_at && d.timestamp <= until)
        .collect();
    if relevant.is_empty() {
        return None;
    }
    relevant.sort_by_key(|d| d.timestamp);

    for candle in relevant {
        let (target_hit, stop_hit) = if tactic.scenario == "bull" {
            (candle.high >= target_price, candle.low <= stop_price)
        } else {
            (candle.low <= target_price, candle.high >= stop_price)
        };

        if stop_hit {
            return Some("stop_hit");
        }
        if target_hit {
            return Some("target_hit");
        }
    }

    Some("neither")
}

/// statystyka trafności - tylko z tactic'ów już zweryfikowanych
pub fn compute_track_record(tactics: &[TrackedTactic]) -> TacticTrackRecord {
    let mut record = TacticTrackRecord {
        verified_24h_total: 0,
        verified_24h_hits: 0,
        verified_7d_total: 0,
        verified_7d_hits: 0,
    };

    for tactic in tactics {
        if let Some(v) = &tactic.verified_24h {
            record.verified_24h_total += 1;
            if v.outcome == "target_hit" {
                record.verified_24h_hits += 1;
            }
        }
        if let Some(v) = &tactic.verified_7d {
            record.verified_7d_total += 1;
            if v.outcome == "target_hit" {
                record.verified_7d_hits += 1;
            }
        }
    }

    record
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(timestamp: i64, high: f64, low: f64) -> MarketData {
        MarketData {
            symbol: "TEST".to_string(),
            time: "2026-01-01".to_string(),
            timestamp,
            open: (high + low) / 2.0,
            high,
            low,
            close: (high + low) / 2.0,
        }
    }

    fn tactic(scenario: &str, reference_price: f64, target_pct: f64, stop_loss_pct: f64) -> TrackedTactic {
        TrackedTactic {
            id: "test".to_string(),
            instrument: "NASDAQ".to_string(),
            scenario: scenario.to_string(),
            reference_price,
            entry_pct: 0.0,
            target_pct,
            stop_loss_pct,
            generated_at: 1000,
            verified_24h: None,
            verified_7d: None,
        }
    }

    #[test]
    fn neutral_scenario_is_never_evaluated() {
        let t = tactic("neutral", 100.0, 2.0, -1.0);
        let history = vec![candle(2000, 110.0, 90.0)];
        assert_eq!(evaluate_outcome(&t, &history, 3000), None);
    }

    #[test]
    fn returns_none_when_no_candles_fall_in_window() {
        let t = tactic("bull", 100.0, 2.0, -1.0);
        let history = vec![candle(500, 110.0, 90.0)]; // przed generated_at
        assert_eq!(evaluate_outcome(&t, &history, 3000), None);
    }

    #[test]
    fn bull_hits_target_when_high_reaches_it() {
        let t = tactic("bull", 100.0, 2.0, -1.0); // target=102.0, stop=99.0
        let history = vec![candle(2000, 103.0, 100.5)];
        assert_eq!(evaluate_outcome(&t, &history, 3000), Some("target_hit"));
    }

    #[test]
    fn bull_hits_stop_when_low_reaches_it() {
        let t = tactic("bull", 100.0, 2.0, -1.0); // target=102.0, stop=99.0
        let history = vec![candle(2000, 100.5, 98.5)];
        assert_eq!(evaluate_outcome(&t, &history, 3000), Some("stop_hit"));
    }

    #[test]
    fn bull_neither_when_price_stays_in_range() {
        let t = tactic("bull", 100.0, 2.0, -1.0);
        let history = vec![candle(2000, 100.8, 99.5)];
        assert_eq!(evaluate_outcome(&t, &history, 3000), Some("neither"));
    }

    #[test]
    fn same_day_double_touch_is_conservatively_counted_as_stop() {
        // gdyby liczyło jako target_hit, dałoby się podkręcić % szerokim target+stop na jednej świecy
        let t = tactic("bull", 100.0, 2.0, -1.0);
        let history = vec![candle(2000, 105.0, 95.0)]; // dotyka i target=102, i stop=99
        assert_eq!(evaluate_outcome(&t, &history, 3000), Some("stop_hit"));
    }

    #[test]
    fn bear_hits_target_when_low_drops_to_it() {
        let t = tactic("bear", 100.0, -2.0, 1.0); // target=98.0, stop=101.0
        let history = vec![candle(2000, 100.5, 97.0)];
        assert_eq!(evaluate_outcome(&t, &history, 3000), Some("target_hit"));
    }

    #[test]
    fn bear_hits_stop_when_high_rises_to_it() {
        let t = tactic("bear", 100.0, -2.0, 1.0); // target=98.0, stop=101.0
        let history = vec![candle(2000, 101.5, 99.5)];
        assert_eq!(evaluate_outcome(&t, &history, 3000), Some("stop_hit"));
    }

    #[test]
    fn earlier_day_outcome_wins_over_later_day() {
        let t = tactic("bull", 100.0, 2.0, -1.0);
        let history = vec![
            candle(2000, 103.0, 100.5), // dzień 1: target hit
            candle(3000, 90.0, 80.0),   // dzień 2: stop by też trafił, ale liczy się dzień 1
        ];
        assert_eq!(evaluate_outcome(&t, &history, 4000), Some("target_hit"));
    }

    #[test]
    fn compute_track_record_counts_only_verified_and_excludes_neutral() {
        use crate::models::TacticVerification;

        let mut hit = tactic("bull", 100.0, 2.0, -1.0);
        hit.verified_24h = Some(TacticVerification { outcome: "target_hit".to_string(), checked_at: 5000 });

        let mut miss = tactic("bear", 100.0, -2.0, 1.0);
        miss.verified_24h = Some(TacticVerification { outcome: "stop_hit".to_string(), checked_at: 5000 });

        let unverified = tactic("bull", 100.0, 2.0, -1.0);

        let record = compute_track_record(&[hit, miss, unverified]);
        assert_eq!(record.verified_24h_total, 2);
        assert_eq!(record.verified_24h_hits, 1);
        assert_eq!(record.verified_7d_total, 0);
    }
}
