import type { AnalyticalReport, PreciousMetalsReport, TechnicalIndicators } from "../types";

/**
 * Metale i equity trzymają dane w dwóch różnych kształtach (`PreciousMetalsReport`
 * ma pola per-metal, equity to lista raportów par "A->B") - to rozgałęzienie
 * powielało się w TickerTape i OverviewView. Jedno miejsce, jeden kształt wyjścia.
 */
export type InstrumentData = {
  price: number;
  changePct: number;
  correlation: number;
  /** z czym liczona jest korelacja - inaczej "0.187" nie znaczy nic */
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
  // symbol ma postać "LEADER->FOLLOWER"
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
