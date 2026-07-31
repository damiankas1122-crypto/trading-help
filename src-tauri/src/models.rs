// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MarketData {
    pub symbol: String,
    pub time: String,
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// RSI/MACD computed from closing prices.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TechnicalIndicators {
    pub rsi: f64,
    pub macd_line: f64,
    pub macd_signal: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnalyticalReport {
    pub symbol: String,
    /// `None` means the correlation was not measured - too few shared sessions.
    /// 0.0 is a valid reading (no linear relationship) and cannot double as a
    /// failure code.
    pub correlation: Option<f64>,
    /// Shared session days the correlation was computed from.
    ///
    /// `#[serde(default)]` is a migration requirement, not a convenience: records
    /// written before this field existed would otherwise fail to deserialize and
    /// take the whole snapshot history file down with them. A stored 0 means
    /// "unknown, pre-measurement record", which is how a zero sample size has to
    /// be read anyway.
    #[serde(default)]
    pub overlapping_observations: usize,
    pub volatility: f64,
    pub technicals: TechnicalIndicators,
    /// Latest close of the leader; feeds the ticker tape.
    pub latest_close: f64,
    pub daily_change_pct: f64,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreciousMetalsReport {
    /// See `AnalyticalReport::correlation` for why this is an `Option`.
    pub correlation: Option<f64>,
    /// Gold and silver futures are the pair whose session calendars actually
    /// diverge, so the sample size behind the correlation matters most here.
    #[serde(default)]
    pub overlapping_observations: usize,
    pub current_gsr: f64,
    pub gsr_30d_ago: f64,
    pub gsr_change_pct: f64,
    pub gold_volatility: f64,
    pub silver_volatility: f64,
    pub gold_technicals: TechnicalIndicators,
    pub silver_technicals: TechnicalIndicators,
    pub gold_price: f64,
    pub silver_price: f64,
    pub gold_daily_change_pct: f64,
    pub silver_daily_change_pct: f64,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewsItem {
    pub title: String,
    pub description: String,
    pub link: String,
    pub published: String,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Citation {
    pub claim: String,
    /// "news" | "numeric"
    pub evidence_type: String,
    pub evidence_label: String,
    pub evidence_link: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstrumentBriefing {
    pub instrument: String,
    pub commentary: String,
    pub sentiment_impact: f64,
    pub pine_script_signal: String,
    pub pine_script_signal_explanation: String,
    pub citations: Vec<Citation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
    pub equity_reports: Vec<AnalyticalReport>,
    pub metals_report: PreciousMetalsReport,
    pub timestamp: String,
}

/// Current quote taken from chart metadata: one light request, no candles.
/// Feeds the ticker tape on a short interval, independently of the heavy
/// analytical context (which stays on its own 5-minute cadence).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveQuote {
    /// Catalogue id, not a Yahoo symbol.
    pub instrument: String,
    pub price: f64,
    pub previous_close: f64,
    pub daily_change_pct: f64,
    /// Unix seconds of the quote itself as reported by the feed - what "now"
    /// means for this figure. Futures arrive delayed, so the UI shows this
    /// time rather than implying the price is from this very second.
    pub market_time: i64,
}

/// Numeric market data: correlations, GSR, templated Pine Scripts. No AI and
/// no rate limit, so it is refreshed independently of per-instrument briefings.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MarketContext {
    pub equity_reports: Vec<AnalyticalReport>,
    pub metals_report: PreciousMetalsReport,
    pub pine_script_correlation: String,
    pub pine_script_correlation_explanation: String,
    pub pine_script_gsr: String,
    pub pine_script_gsr_explanation: String,
    pub timestamp: String,
}

/// Generated on demand, separately from briefings. Percentages are relative
/// to `reference_price`, not absolute price levels.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradingTactic {
    pub instrument: String,
    pub scenario: String,
    pub reasoning: String,
    pub entry_pct: f64,
    pub target_pct: f64,
    pub stop_loss_pct: f64,
    pub reference_price: f64,
    pub disclaimer: String,
    pub timestamp: String,
}

/// Outcome of one verification window (24h or 7d). Written once, never
/// overwritten.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TacticVerification {
    /// "target_hit" | "stop_hit" | "neither"
    pub outcome: String,
    pub checked_at: i64,
}

/// Tactic tracked for accuracy. `reference_price` is the price at generation
/// time.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackedTactic {
    pub id: String,
    pub instrument: String,
    pub scenario: String,
    pub reference_price: f64,
    pub entry_pct: f64,
    pub target_pct: f64,
    pub stop_loss_pct: f64,
    pub generated_at: i64,
    #[serde(default)]
    pub verified_24h: Option<TacticVerification>,
    #[serde(default)]
    pub verified_7d: Option<TacticVerification>,
}

/// Aggregated accuracy statistics.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TacticTrackRecord {
    pub verified_24h_total: u32,
    pub verified_24h_hits: u32,
    pub verified_7d_total: u32,
    pub verified_7d_hits: u32,
    /// Stored tactics excluded from the statistics because their reference price
    /// cannot be scored. Reported separately so a smaller sample size is
    /// explained rather than silently smaller.
    pub skipped_invalid_reference_price: u32,
}
