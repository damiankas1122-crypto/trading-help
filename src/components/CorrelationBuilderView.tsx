import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AnalyticalReport } from "../types";
import { Panel } from "./Panel";
import { formatErrorMessage } from "../utils/format";

export function CorrelationBuilderView() {
  const [tickerA, setTickerA] = useState("");
  const [tickerB, setTickerB] = useState("");
  const [result, setResult] = useState<AnalyticalReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const compute = async () => {
    if (!tickerA.trim() || !tickerB.trim()) {
      setError("Podaj oba tickery.");
      return;
    }
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const report = await invoke<AnalyticalReport>("get_custom_pair_correlation", {
        tickerA: tickerA.trim(),
        tickerB: tickerB.trim(),
      });
      setResult(report);
    } catch (err) {
      setError(formatErrorMessage(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-3">
      <Panel title="Konstruktor par korelacji">
        <p className="text-xs text-term-dim mb-3">
          Dowolna para tickerów Yahoo Finance (np. AAPL, ^IXIC, GC=F) - niezależnie od watchlisty.
        </p>
        <div className="flex flex-wrap items-end gap-2">
          <label className="text-xs font-mono">
            <span className="block text-term-faint uppercase tracking-wide mb-1">Ticker A</span>
            <input
              value={tickerA}
              onChange={(e) => setTickerA(e.target.value)}
              placeholder="np. AAPL"
              className="bg-black border border-term-line-strong px-2.5 py-1.5 text-sm text-term-text focus:outline-none focus:border-term-amber w-32"
            />
          </label>
          <label className="text-xs font-mono">
            <span className="block text-term-faint uppercase tracking-wide mb-1">Ticker B</span>
            <input
              value={tickerB}
              onChange={(e) => setTickerB(e.target.value)}
              placeholder="np. MSFT"
              className="bg-black border border-term-line-strong px-2.5 py-1.5 text-sm text-term-text focus:outline-none focus:border-term-amber w-32"
            />
          </label>
          <button
            onClick={compute}
            disabled={loading}
            className="px-4 py-1.5 border border-term-amber text-term-amber text-xs font-bold uppercase tracking-wide hover:bg-term-amber/10 disabled:opacity-40 transition-colors"
          >
            {loading ? "Liczę..." : "Oblicz"}
          </button>
        </div>
        {error && <p className="text-xs text-term-red font-mono mt-3">{error}</p>}
      </Panel>

      {result && (() => {
        // Labels come from the result, not from the live inputs: editing the
        // form after computing must not relabel numbers that belong to the
        // previous pair. The symbol has the form "A->B".
        const baseTicker = result.symbol.split("->")[0] ?? result.symbol;
        return (
        <Panel title={result.symbol}>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs font-mono">
            <div>
              <span className="block text-term-faint uppercase tracking-wide">Korelacja pary</span>
              <span className="text-term-text font-semibold tabular-nums">
                {result.correlation === null ? "—" : result.correlation.toFixed(4)}
              </span>
            </div>
            <div>
              <span className="block text-term-faint uppercase tracking-wide">Wspólne sesje</span>
              <span className="text-term-text font-semibold tabular-nums">{result.overlapping_observations}</span>
            </div>
            <div>
              <span className="block text-term-faint uppercase tracking-wide">Zmienność {baseTicker}</span>
              <span className="text-term-text font-semibold tabular-nums">{result.volatility.toFixed(4)}</span>
            </div>
            <div>
              <span className="block text-term-faint uppercase tracking-wide">RSI (14) {baseTicker}</span>
              <span className="text-term-text font-semibold tabular-nums">{result.technicals.rsi.toFixed(2)}</span>
            </div>
            <div>
              <span className="block text-term-faint uppercase tracking-wide">MACD {baseTicker}</span>
              <span className="text-term-text font-semibold tabular-nums">
                {result.technicals.macd_line.toFixed(4)} (sygnał {result.technicals.macd_signal.toFixed(4)})
              </span>
            </div>
          </div>
          {result.correlation === null && (
            <p className="text-[11px] text-term-amber mt-3">
              Korelacji nie zmierzono — te dwa instrumenty mają za mało wspólnych sesji w oknie
              90 dni ({result.overlapping_observations}).
            </p>
          )}
          <p className="text-[11px] text-term-faint mt-3">
            Korelacja dotyczy obu tickerów. Pozostałe wskaźniki liczone są wyłącznie dla{" "}
            {baseTicker} — drugi ticker wchodzi tylko do korelacji.
          </p>
        </Panel>
        );
      })()}
    </div>
  );
}
