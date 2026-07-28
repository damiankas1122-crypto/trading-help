import type { AnalyticalReport, PreciousMetalsReport } from "../types";
import { INSTRUMENTS } from "../constants";
import { signedPct } from "../utils/format";
import { instrumentDataFor } from "../utils/instrumentData";

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
        const data = instrumentDataFor(instrument, equityReports, metalsReport);
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
