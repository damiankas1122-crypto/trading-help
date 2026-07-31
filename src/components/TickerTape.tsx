import type { AnalyticalReport, LiveQuote, PreciousMetalsReport } from "../types";
import { INSTRUMENTS } from "../constants";
import { signedPct, formatUnixDateTime } from "../utils/format";
import { instrumentDataFor } from "../utils/instrumentData";

export function TickerTape({
  equityReports,
  metalsReport,
  liveQuotes,
  quotesSettled,
  archivalTimestamp,
}: {
  equityReports: AnalyticalReport[] | null;
  metalsReport: PreciousMetalsReport | null;
  /** Keyed by instrument id; a missing key falls back to the analytical context. */
  liveQuotes: Record<string, LiveQuote>;
  /** Whether the first quote round has finished, whatever its outcome. */
  quotesSettled: boolean;
  /** Set when the numbers below come from a stored snapshot, not a live fetch. */
  archivalTimestamp: string | null;
}) {
  // The newest quote time is the honest "as of" for the whole tape; futures
  // arrive delayed, so this shows what the feed reported, not the wall clock.
  const latestQuoteTime = Object.values(liveQuotes).reduce(
    (max, quote) => Math.max(max, quote.market_time),
    0
  );

  return (
    <div className="bg-black border-b border-term-line px-4 py-1.5 flex flex-wrap items-baseline gap-x-7 gap-y-1 font-mono text-xs tabular-nums">
      {INSTRUMENTS.map((instrument) => {
        const live = liveQuotes[instrument] ?? null;
        const data = instrumentDataFor(instrument, equityReports, metalsReport);
        const price = live ? live.price : data?.price ?? null;
        const changePct = live ? live.daily_change_pct : data?.changePct ?? null;
        return (
          <span key={instrument}>
            <span className="text-term-faint mr-2">{instrument}</span>
            {price !== null && changePct !== null ? (
              <>
                <span className="text-term-text">{price.toFixed(2)}</span>{" "}
                <span className={changePct >= 0 ? "text-term-green" : "text-term-red"}>
                  {changePct >= 0 ? "▲" : "▼"} {signedPct(changePct)}
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
      {/* Three states, in order of precedence: live figures win; while the
          first round is in flight the tape says so rather than judging the
          data; only a finished round with nothing to show earns the warning,
          which then stays until quotes arrive - deliberately not dismissible,
          since hiding it would leave stale prices looking current. */}
      {latestQuoteTime > 0 ? (
        <span className="text-term-faint ml-auto">notowania {formatUnixDateTime(latestQuoteTime)}</span>
      ) : !quotesSettled ? (
        <span className="text-term-faint ml-auto">pobieram notowania…</span>
      ) : archivalTimestamp ? (
        <span className="text-term-amber ml-auto">
          DANE ARCHIWALNE {formatUnixDateTime(archivalTimestamp)}
        </span>
      ) : null}
    </div>
  );
}
