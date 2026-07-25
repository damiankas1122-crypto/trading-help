import type { Snapshot } from "../types";

/** zamiast pustego ekranu pokazujemy ostatni zapisany odczyt, jeśli jest */
export function LastSnapshotPreview({ snapshot, onRefresh }: { snapshot: Snapshot; onRefresh: () => void }) {
  return (
    <div className="bg-[#0a0a1a]/40 rounded-2xl border border-cyan-900/20 p-6 opacity-80">
      <div className="flex items-center justify-between mb-4 flex-wrap gap-2">
        <span className="text-slate-400 text-xs font-bold uppercase tracking-[0.15em]">
          Ostatnia zapisana analiza — {snapshot.slot}
        </span>
        <button
          onClick={onRefresh}
          className="text-xs text-cyan-400 hover:text-cyan-300 font-mono underline underline-offset-2"
        >
          Odśwież teraz
        </button>
      </div>
      <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-xs font-mono text-slate-500">
        {snapshot.equity_reports.map((r) => (
          <div key={r.symbol}>
            <span>{r.symbol}: </span>
            <span className="text-slate-400">{r.correlation.toFixed(3)}</span>
          </div>
        ))}
        <div>
          <span>GSR: </span>
          <span className="text-slate-400">{snapshot.metals_report.current_gsr.toFixed(2)}</span>
        </div>
      </div>
      <p className="text-slate-600 text-xs font-mono mt-4">
        Wybierz porę dnia powyżej, żeby wygenerować świeżą analizę z aktualnymi komentarzami.
      </p>
    </div>
  );
}
