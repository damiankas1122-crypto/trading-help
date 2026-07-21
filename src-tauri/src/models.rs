// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MarketData {
    pub symbol: String,
    pub time: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnalyticalReport {
    pub symbol: String,
    pub correlation: f64,
    pub volatility: f64,
    pub sentiment_impact: f64,
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
pub struct InstrumentBriefing {
    pub instrument: String,
    pub commentary: String,
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