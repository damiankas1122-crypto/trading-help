import type { AnalyticalReport, PreciousMetalsReport, TechnicalIndicators } from "../types";

/**
 * Metals and equities store data in two different shapes: `PreciousMetalsReport`
 * has per-metal fields, while equities come as a list of "A->B" pair reports.
 * That branch was duplicated in TickerTape and OverviewView; this is the single
 * place for it, with one output shape.
 */
export type InstrumentData = {
  price: number;
  changePct: number;
  correlation: number;
  /** What the correlation is measured against; "0.187" alone means nothing. */
  correlatedWith: string | null;
  volatility: number;
  technicals: TechnicalIndicators;
};

export function isMetal(instrument: string): boolean {
  return instrument === "GOLD" || instrument === "SILVER";
}

export function instrumentDataFor(
  instrument: string,
  equityReports: AnalyticalReport[] | null,
  metalsReport: PreciousMetalsReport | null
): InstrumentData | null {
  if (isMetal(instrument)) {
    if (!metalsReport) return null;
    const isGold = instrument === "GOLD";
    return {
      price: isGold ? metalsReport.gold_price : metalsReport.silver_price,
      changePct: isGold ? metalsReport.gold_daily_change_pct : metalsReport.silver_daily_change_pct,
      correlation: metalsReport.correlation,
      correlatedWith: isGold ? "SILVER" : "GOLD",
      volatility: isGold ? metalsReport.gold_volatility : metalsReport.silver_volatility,
      technicals: isGold ? metalsReport.gold_technicals : metalsReport.silver_technicals,
    };
  }

  const report = equityReports?.find((r) => r.symbol.startsWith(`${instrument}->`));
  if (!report) return null;
  // The symbol has the form "LEADER->FOLLOWER".
  const follower = report.symbol.split("->")[1] ?? null;
  return {
    price: report.latest_close,
    changePct: report.daily_change_pct,
    correlation: report.correlation,
    correlatedWith: follower,
    volatility: report.volatility,
    technicals: report.technicals,
  };
}
