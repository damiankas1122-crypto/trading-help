import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TradingTactic } from "../types";
import { TACTIC_SCENARIO_STYLE, TACTIC_SCENARIO_LABEL } from "../constants";
import { signedPct, formatErrorMessage } from "../utils/format";
import { cancelOperation, isCancellation, newOperationId } from "../utils/aiOperations";

// `tactic` is controlled (lifted into App.tsx per instrument); otherwise
// switching focus would discard a generated tactic and force another Gemini
// call against the 5/min, 20/day limit.
export function TradingTacticSection({
  instrument,
  tactic,
  onTacticChange,
}: {
  instrument: string;
  tactic: TradingTactic | null;
  onTacticChange: (tactic: TradingTactic) => void;
}) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [operationId, setOperationId] = useState<string | null>(null);

  const generate = async () => {
    const id = newOperationId();
    setOperationId(id);
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<TradingTactic>("generate_trading_tactic", {
        instrument,
        operationId: id,
      });
      onTacticChange(result);
    } catch (err) {
      // Cancelling returns to idle: no error panel, nothing to explain.
      if (!isCancellation(err)) {
        setError(formatErrorMessage(err));
      }
    } finally {
      setLoading(false);
      setOperationId(null);
    }
  };

  return (
    <div className="space-y-3">
      {!tactic && (
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={generate}
            disabled={loading}
            className="text-xs font-mono text-term-amber hover:text-term-text border border-term-line px-3 py-2 disabled:opacity-50 transition-colors"
          >
            {loading ? "Generuję taktykę..." : "Wygeneruj taktykę tradingową"}
          </button>
          {loading && operationId && (
            <button
              onClick={() => cancelOperation(operationId)}
              className="text-xs font-mono text-term-dim hover:text-term-red border border-term-line-strong px-3 py-2 transition-colors"
            >
              Przerwij
            </button>
          )}
        </div>
      )}
      {error && <p className="text-xs text-term-red font-mono">{error}</p>}
      {tactic && (
        <div className={`border p-4 space-y-3 ${TACTIC_SCENARIO_STYLE[tactic.scenario] ?? TACTIC_SCENARIO_STYLE.neutral}`}>
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-[0.15em]">
              {TACTIC_SCENARIO_LABEL[tactic.scenario] ?? tactic.scenario}
            </span>
            {loading && operationId ? (
              // Regenerating is the same Gemini call, so it gets the same way out.
              <button
                onClick={() => cancelOperation(operationId)}
                className="text-xs text-term-dim hover:text-term-red underline underline-offset-2"
              >
                Przerwij
              </button>
            ) : (
              <button
                onClick={generate}
                disabled={loading}
                className="text-xs text-term-dim hover:text-term-amber underline underline-offset-2 disabled:opacity-50"
              >
                {loading ? "..." : "Odśwież"}
              </button>
            )}
          </div>
          <p className="text-sm leading-relaxed">{tactic.reasoning}</p>
          <div className="grid grid-cols-3 gap-2 text-xs font-mono">
            <div>Wejście: <span className="opacity-90">po bieżącej cenie</span></div>
            <div>Cel: <span className="opacity-90">{signedPct(tactic.target_pct)}</span></div>
            <div>Stop: <span className="opacity-90">{signedPct(tactic.stop_loss_pct)}</span></div>
          </div>
          <p className="text-xs opacity-70 font-mono tabular-nums">
            Poziomy liczone względem ceny {tactic.reference_price.toFixed(2)} z momentu generacji.
          </p>
          <p className="text-xs opacity-80 font-mono border-t border-current/20 pt-2">{tactic.disclaimer}</p>
        </div>
      )}
    </div>
  );
}
