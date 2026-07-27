import type { AnalyticalReport, PreciousMetalsReport } from "../types";
import { INSTRUMENTS } from "../constants";
import { signedPct } from "../utils/format";

function priceFor(
  instrument: string,
  equityReports: AnalyticalReport[] | null,
  metalsReport: PreciousMetalsReport | null
): { price: number; changePct: number } | null {
  if (instrument === "GOLD" || instrument === "SILVER") {
    if (!metalsReport) return null;
    return instrument === "GOLD"
      ? { price: metalsReport.gold_price, changePct: metalsReport.gold_daily_change_pct }
      : { price: metalsReport.silver_price, changePct: metalsReport.silver_daily_change_pct };
  }
  const report = equityReports?.find((r) => r.symbol.startsWith(`${instrument}->`));
  return report ? { price: report.latest_close, changePct: report.daily_change_pct } : null;
}

export function TickerTape({
  equityReports,
  metalsReport,
}: {
  equityReports: AnalyticalReport[] | null;
  metalsReport: PreciousMetalsReport | null;
}) {
  return (
    <div className="bg-black border-b border-term-line px-4 py-1.5 flex flex-wrap gap-x-7 gap-y-1 font-mono text-xs tabular-nums">
      {INSTRUMENTS.map((instrument) => {
        const data = priceFor(instrument, equityReports, metalsReport);
        return (
          <span key={instrument}>
            <span className="text-term-faint mr-2">{instrument}</span>
            {data ? (
              <>
                <span className="text-term-text">{data.price.toFixed(2)}</span>{" "}
                <span className={data.changePct >= 0 ? "text-term-green" : "text-term-red"}>
                  {data.changePct >= 0 ? "▲" : "▼"} {signedPct(data.changePct)}
                </span>
              </>
            ) : (
              <span className="text-term-faint">—</span>
            )}
          </span>
        );
      })}
      {metalsReport && (
        <span>
          <span className="text-term-faint mr-2">GSR</span>
          <span className="text-term-amber">{metalsReport.current_gsr.toFixed(2)}</span>
        </span>
      )}
    </div>
  );
}
