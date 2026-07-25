import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TradingTactic } from "../types";
import { TACTIC_SCENARIO_STYLE, TACTIC_SCENARIO_LABEL } from "../constants";
import { signedPct } from "../utils/format";

export function TradingTacticSection({ instrument }: { instrument: string }) {
  const [tactic, setTactic] = useState<TradingTactic | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generate = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<TradingTactic>("generate_trading_tactic", { instrument });
      setTactic(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-3">
      {!tactic && (
        <button
          onClick={generate}
          disabled={loading}
          className="text-xs font-mono text-cyan-400 hover:text-cyan-300 border border-cyan-900/40 rounded-lg px-3 py-2 disabled:opacity-50 transition-colors"
        >
          {loading ? "Generuję taktykę..." : "Wygeneruj taktykę tradingową"}
        </button>
      )}
      {error && <p className="text-xs text-red-400 font-mono">{error}</p>}
      {tactic && (
        <div className={`rounded-2xl border p-4 space-y-3 ${TACTIC_SCENARIO_STYLE[tactic.scenario] ?? TACTIC_SCENARIO_STYLE.neutral}`}>
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-[0.15em]">
              {TACTIC_SCENARIO_LABEL[tactic.scenario] ?? tactic.scenario}
            </span>
            <button
              onClick={generate}
              disabled={loading}
              className="text-xs text-slate-400 hover:text-cyan-300 underline underline-offset-2 disabled:opacity-50"
            >
              {loading ? "..." : "Odśwież"}
            </button>
          </div>
          <p className="text-sm text-slate-300 leading-relaxed">{tactic.reasoning}</p>
          <div className="grid grid-cols-3 gap-2 text-xs font-mono text-slate-400">
            <div>Wejście: <span className="text-slate-200">po bieżącej cenie</span></div>
            <div>Cel: <span className="text-slate-200">{signedPct(tactic.target_pct)}</span></div>
            <div>Stop: <span className="text-slate-200">{signedPct(tactic.stop_loss_pct)}</span></div>
          </div>
          <p className="text-xs text-amber-400/90 font-mono border-t border-white/10 pt-2">{tactic.disclaimer}</p>
        </div>
      )}
    </div>
  );
}
