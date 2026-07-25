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
  disclaimer: string;
  timestamp: string;
};

export type TacticTrackRecord = {
  verified_24h_total: number;
  verified_24h_hits: number;
  verified_7d_total: number;
  verified_7d_hits: number;
};

export type BriefingProgress = {
  instrument: string;
  step: number;
  total: number;
};

export type FullBriefing = {
  slot: string;
  compared_to: string | null;
  equity_reports: AnalyticalReport[];
  metals_report: PreciousMetalsReport;
  instrument_briefings: InstrumentBriefing[];
  pine_script_correlation: string;
  pine_script_correlation_explanation: string;
  pine_script_gsr: string;
  pine_script_gsr_explanation: string;
  is_stale_data: boolean;
  stale_data_message: string | null;
};

export type Snapshot = {
  equity_reports: AnalyticalReport[];
  metals_report: PreciousMetalsReport;
  timestamp: string;
  slot: string;
};

export type UpdateStatus = "idle" | "available" | "downloading" | "ready" | "error";
