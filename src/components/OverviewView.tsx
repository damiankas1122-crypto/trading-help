import type { MarketContext, Snapshot, InstrumentBriefing } from "../types";
import { PineScriptSection } from "./PineScriptSection";
import { CitationsSection } from "./CitationsSection";
import { LastSnapshotPreview } from "./LastSnapshotPreview";
import { EmptyStateFirstRun } from "./EmptyStateFirstRun";
import { Panel } from "./Panel";
import { signedPct } from "../utils/format";

function numericDataFor(instrument: string, marketContext: MarketContext) {
  if (instrument === "GOLD" || instrument === "SILVER") {
    const m = marketContext.metals_report;
    const isGold = instrument === "GOLD";
    return {
      price: isGold ? m.gold_price : m.silver_price,
      changePct: isGold ? m.gold_daily_change_pct : m.silver_daily_change_pct,
      correlation: m.correlation,
      volatility: isGold ? m.gold_volatility : m.silver_volatility,
      technicals: isGold ? m.gold_technicals : m.silver_technicals,
    };
  }
  const report = marketContext.equity_reports.find((r) => r.symbol.startsWith(`${instrument}->`));
  if (!report) return null;
  return {
    price: report.latest_close,
    changePct: report.daily_change_pct,
    correlation: report.correlation,
    volatility: report.volatility,
    technicals: report.technicals,
  };
}

function Kv({ label, value, className = "" }: { label: string; value: string; className?: string }) {
  return (
    <div className="flex justify-between py-1 border-b border-dotted border-term-line last:border-b-0 text-xs">
      <span className="text-term-dim">{label}</span>
      <span className={`font-semibold tabular-nums ${className}`}>{value}</span>
    </div>
  );
}

export function OverviewView({
  instrument,
  marketContext,
  marketContextError,
  lastSnapshot,
  onRefreshMarketContext,
  instrumentBriefing,
  briefingLoading,
  briefingError,
  onAnalyze,
}: {
  instrument: string;
  marketContext: MarketContext | null;
  marketContextError: string | null;
  lastSnapshot: Snapshot | null;
  onRefreshMarketContext: () => void;
  instrumentBriefing: InstrumentBriefing | null;
  briefingLoading: boolean;
  briefingError: string | null;
  onAnalyze: () => void;
}) {
  const numeric = marketContext ? numericDataFor(instrument, marketContext) : null;

  if (!marketContext && !lastSnapshot) {
    return marketContextError ? (
      <div className="border border-term-red/50 bg-term-red/10 p-3 text-term-red text-xs font-mono whitespace-pre-wrap">
        {marketContextError}
      </div>
    ) : (
      <EmptyStateFirstRun />
    );
  }

  return (
    <div className="space-y-3">
      {!marketContext && lastSnapshot && (
        <LastSnapshotPreview snapshot={lastSnapshot} onRefresh={onRefreshMarketContext} />
      )}

      <Panel
        title="AI Co-Pilot Engine"
        badge={
          <button
            onClick={onRefreshMarketContext}
            className="text-[10px] normal-case tracking-normal text-term-faint hover:text-term-amber underline underline-offset-2"
          >
            Odśwież dane rynkowe
          </button>
        }
      >
        <p className="text-xs text-term-dim mb-3">
          Analiza jednego instrumentu na żądanie - jedno wywołanie AI, bez czekania na resztę watchlisty.
        </p>
        <button
          onClick={onAnalyze}
          disabled={briefingLoading}
          className="px-4 py-2 border border-term-amber text-term-amber text-xs font-bold uppercase tracking-wide hover:bg-term-amber/10 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        >
          {briefingLoading ? `Analizuję ${instrument}...` : `Analizuj ${instrument}`}
        </button>
      </Panel>

      {briefingError && (
        <div className="border border-term-red/50 bg-term-red/10 p-3 text-term-red text-xs font-mono whitespace-pre-wrap">
          {briefingError}
        </div>
      )}

      {marketContext && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <Panel title={`${instrument} // Dane liczbowe`}>
            {numeric ? (
              <>
                <Kv label="Cena" value={numeric.price.toFixed(2)} />
                <Kv
                  label="Zmiana dzienna"
                  value={signedPct(numeric.changePct)}
                  className={numeric.changePct >= 0 ? "text-term-green" : "text-term-red"}
                />
                <Kv label="RSI (14)" value={numeric.technicals.rsi.toFixed(1)} />
                <Kv
                  label="MACD linia"
                  value={numeric.technicals.macd_line.toFixed(2)}
                  className={numeric.technicals.macd_line >= 0 ? "text-term-green" : "text-term-red"}
                />
                <Kv label="MACD sygnał" value={numeric.technicals.macd_signal.toFixed(2)} />
                <Kv label="Korelacja" value={numeric.correlation.toFixed(3)} />
                <Kv label="Zmienność" value={numeric.volatility.toFixed(3)} />
                {(instrument === "GOLD" || instrument === "SILVER") && (
                  <Kv label="GSR" value={marketContext.metals_report.current_gsr.toFixed(2)} />
                )}
              </>
            ) : (
              <p className="text-xs text-term-faint">Brak danych liczbowych.</p>
            )}
          </Panel>

          <Panel title="Briefing AI">
            {instrumentBriefing ? (
              <p className="text-xs text-term-dim whitespace-pre-wrap leading-relaxed">
                {instrumentBriefing.commentary}
              </p>
            ) : (
              <p className="text-xs text-term-faint">
                Kliknij "Analizuj {instrument}", żeby wygenerować briefing AI dla tego instrumentu.
              </p>
            )}
          </Panel>
        </div>
      )}

      {instrumentBriefing && instrumentBriefing.citations.length > 0 && (
        <CitationsSection citations={instrumentBriefing.citations} />
      )}

      {instrumentBriefing && (
        <PineScriptSection
          title={`Pine Script // ${instrument}`}
          explanation={instrumentBriefing.pine_script_signal_explanation}
          code={instrumentBriefing.pine_script_signal}
        />
      )}

      {marketContext && (
        <>
          <Panel title="Kontekst rynkowy">
            <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-xs font-mono text-term-dim">
              {marketContext.equity_reports.map((r) => (
                <div key={r.symbol}>
                  <span className="text-term-faint">{r.symbol}: </span>
                  <span>{r.correlation.toFixed(3)}</span>
                </div>
              ))}
              <div>
                <span className="text-term-faint">GSR: </span>
                <span>{marketContext.metals_report.current_gsr.toFixed(2)}</span>
              </div>
              <div>
                <span className="text-term-faint">Au-Ag corr: </span>
                <span>{marketContext.metals_report.correlation.toFixed(3)}</span>
              </div>
            </div>
          </Panel>

          <PineScriptSection
            title="Pine Script: Korelacja indeksów"
            explanation={marketContext.pine_script_correlation_explanation}
            code={marketContext.pine_script_correlation}
          />
          <PineScriptSection
            title="Pine Script: Gold/Silver Ratio"
            explanation={marketContext.pine_script_gsr_explanation}
            code={marketContext.pine_script_gsr}
          />
        </>
      )}
    </div>
  );
}
