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

/// RSI/MACD liczone z cen zamknięcia
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TechnicalIndicators {
    pub rsi: f64,
    pub macd_line: f64,
    pub macd_signal: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnalyticalReport {
    pub symbol: String,
    pub correlation: f64,
    pub volatility: f64,
    pub technicals: TechnicalIndicators,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreciousMetalsReport {
    pub correlation: f64,
    pub current_gsr: f64,
    pub gsr_30d_ago: f64,
    pub gsr_change_pct: f64,
    pub gold_volatility: f64,
    pub silver_volatility: f64,
    pub gold_technicals: TechnicalIndicators,
    pub silver_technicals: TechnicalIndicators,
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
    pub slot: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FullBriefing {
    pub slot: String,
    pub compared_to: Option<String>,
    pub equity_reports: Vec<AnalyticalReport>,
    pub metals_report: PreciousMetalsReport,
    pub instrument_briefings: Vec<InstrumentBriefing>,
    pub pine_script_correlation: String,
    pub pine_script_correlation_explanation: String,
    pub pine_script_gsr: String,
    pub pine_script_gsr_explanation: String,
    #[serde(default)]
    pub is_stale_data: bool,
    #[serde(default)]
    pub stale_data_message: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BriefingProgress {
    pub instrument: String,
    pub step: u32,
    pub total: u32,
}

/// generowana na żądanie (osobny przycisk, nie briefing). pct-y to % względem
/// ceny w momencie generacji, nie realna cena
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

/// wynik jednej weryfikacji (24h albo 7d). raz zapisany - nigdy nie
/// nadpisujemy
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TacticVerification {
    /// "target_hit" | "stop_hit" | "neither"
    pub outcome: String,
    pub checked_at: i64,
}

/// taktyka do śledzenia trafności. reference_price - cena w momencie
/// generacji
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

/// statystyka trafności 
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TacticTrackRecord {
    pub verified_24h_total: u32,
    pub verified_24h_hits: u32,
    pub verified_7d_total: u32,
    pub verified_7d_hits: u32,
}
