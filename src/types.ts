export type TechnicalIndicators = {
  rsi: number;
  macd_line: number;
  macd_signal: number;
};

export type AnalyticalReport = {
  symbol: string;
  correlation: number;
  volatility: number;
  technicals: TechnicalIndicators;
  latest_close: number;
  daily_change_pct: number;
  timestamp: string;
};

export type PreciousMetalsReport = {
  correlation: number;
  current_gsr: number;
  gsr_30d_ago: number;
  gsr_change_pct: number;
  gold_volatility: number;
  silver_volatility: number;
  gold_technicals: TechnicalIndicators;
  silver_technicals: TechnicalIndicators;
  gold_price: number;
  silver_price: number;
  gold_daily_change_pct: number;
  silver_daily_change_pct: number;
  timestamp: string;
};

export type Citation = {
  claim: string;
  evidence_type: string;
  evidence_label: string;
  evidence_link: string | null;
};

export type InstrumentBriefing = {
  instrument: string;
  commentary: string;
  sentiment_impact: number;
  pine_script_signal: string;
  pine_script_signal_explanation: string;
  citations: Citation[];
};

export type TradingTactic = {
  instrument: string;
  scenario: string;
  reasoning: string;
  entry_pct: number;
  target_pct: number;
  stop_loss_pct: number;
  /** Price at generation time; target and stop are percentages of it. */
  reference_price: number;
  disclaimer: string;
  timestamp: string;
};

export type TacticTrackRecord = {
  verified_24h_total: number;
  verified_24h_hits: number;
  verified_7d_total: number;
  verified_7d_hits: number;
  /** Stored tactics left out of the statistics because their reference price cannot be scored. */
  skipped_invalid_reference_price: number;
};

export type MarketContext = {
  equity_reports: AnalyticalReport[];
  metals_report: PreciousMetalsReport;
  pine_script_correlation: string;
  pine_script_correlation_explanation: string;
  pine_script_gsr: string;
  pine_script_gsr_explanation: string;
  timestamp: string;
};

export type Snapshot = {
  equity_reports: AnalyticalReport[];
  metals_report: PreciousMetalsReport;
  timestamp: string;
};

export type UpdateStatus = "idle" | "available" | "downloading" | "ready" | "error";

export type ViewId = "przeglad" | "taktyka" | "korelacje" | "skrypty" | "ustawienia";
