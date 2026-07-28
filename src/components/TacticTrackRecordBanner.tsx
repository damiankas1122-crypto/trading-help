import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TacticTrackRecord } from "../types";
import { hitRatePct } from "../utils/format";

// Renders nothing until at least one tactic is verified; otherwise it would
// imply a misleading "0%".
export function TacticTrackRecordBanner() {
  const [record, setRecord] = useState<TacticTrackRecord | null>(null);

  useEffect(() => {
    invoke<TacticTrackRecord>("get_tactic_track_record")
      .then(setRecord)
      .catch(() => setRecord(null));
  }, []);

  if (!record || (record.verified_24h_total === 0 && record.verified_7d_total === 0)) {
    return null;
  }

  return (
    <div className="border border-term-line bg-term-panel px-4 py-2.5 flex flex-wrap items-center gap-x-6 gap-y-1 text-xs font-mono">
      <span className="text-term-faint uppercase tracking-wide">Skuteczność wygenerowanych taktyk:</span>
      <span className="text-term-dim">
        24h: <span className="text-term-amber">{hitRatePct(record.verified_24h_hits, record.verified_24h_total)}</span>
      </span>
      <span className="text-term-dim">
        7 dni: <span className="text-term-amber">{hitRatePct(record.verified_7d_hits, record.verified_7d_total)}</span>
      </span>
    </div>
  );
}
