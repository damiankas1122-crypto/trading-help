import type { Snapshot } from "../types";
import { Panel } from "./Panel";
import { formatCorrelation, formatUnixDateTime } from "../utils/format";

/** Shows the last stored reading, if any, instead of an empty screen. */
export function LastSnapshotPreview({ snapshot, onRefresh }: { snapshot: Snapshot; onRefresh: () => void }) {
  return (
    <Panel
      title="Ostatnie zapisane dane rynkowe"
      badge={
        <button
          onClick={onRefresh}
          className="text-[10px] normal-case tracking-normal text-term-amber hover:text-term-text font-mono underline underline-offset-2"
        >
          Odśwież teraz
        </button>
      }
    >
      <p className="text-term-amber text-xs font-mono mb-3">
        Dane archiwalne, zapisane {formatUnixDateTime(snapshot.timestamp)} — nie odzwierciedlają
        bieżącego rynku.
      </p>
      <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-xs font-mono text-term-faint">
        {snapshot.equity_reports.map((r) => (
          <div key={r.symbol}>
            <span>{r.symbol}: </span>
            <span className="text-term-dim">{formatCorrelation(r.correlation)}</span>
          </div>
        ))}
        <div>
          <span>GSR: </span>
          <span className="text-term-dim">{snapshot.metals_report.current_gsr.toFixed(2)}</span>
        </div>
      </div>
      <p className="text-term-faint text-xs font-mono mt-4">
        Świeże dane rynkowe pobierają się automatycznie. Wybierz instrument i kliknij
        "Analizuj", żeby wygenerować dla niego świeży briefing AI.
      </p>
    </Panel>
  );
}
