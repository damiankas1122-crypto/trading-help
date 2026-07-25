import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TacticTrackRecord } from "../types";
import { hitRatePct } from "../utils/format";

// nic nie pokazujemy dopóki żadna taktyka nie jest zweryfikowana - inaczej
// sugerowalibyśmy fałszywe "0%"
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
    <div className="bg-[#0a0a1a]/60 rounded-2xl border border-cyan-900/30 px-5 py-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-xs font-mono">
      <span className="text-slate-500 uppercase tracking-wide">Skuteczność wygenerowanych taktyk:</span>
      <span className="text-slate-300">
        24h: <span className="text-cyan-300">{hitRatePct(record.verified_24h_hits, record.verified_24h_total)}</span>
      </span>
      <span className="text-slate-300">
        7 dni: <span className="text-cyan-300">{hitRatePct(record.verified_7d_hits, record.verified_7d_total)}</span>
      </span>
    </div>
  );
}
