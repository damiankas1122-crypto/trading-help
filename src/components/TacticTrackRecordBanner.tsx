import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TacticTrackRecord } from "../types";
import { hitRatePct } from "../utils/format";

// Polish plural: 1 takes the accusative singular, 2-4 the plural, everything
// else (including the teens) the genitive plural.
function tacticsPlural(count: number): string {
  if (count === 1) return "taktykę";
  const lastTwo = count % 100;
  const last = count % 10;
  const isFewForm = last >= 2 && last <= 4 && !(lastTwo >= 12 && lastTwo <= 14);
  return isFewForm ? "taktyki" : "taktyk";
}

// Renders nothing until at least one tactic is verified; otherwise it would
// imply a misleading "0%".
export function TacticTrackRecordBanner() {
  const [record, setRecord] = useState<TacticTrackRecord | null>(null);

  useEffect(() => {
    invoke<TacticTrackRecord>("get_tactic_track_record")
      .then(setRecord)
      .catch(() => setRecord(null));
  }, []);

  const skipped = record?.skipped_invalid_reference_price ?? 0;
  const hasVerified = record !== null && (record.verified_24h_total > 0 || record.verified_7d_total > 0);

  // The skipped count is also a reason to render: hiding it would leave the
  // shrunken sample size unexplained, which is the very thing it exists to say.
  if (!record || (!hasVerified && skipped === 0)) {
    return null;
  }

  return (
    <div className="border border-term-line bg-term-panel px-4 py-2.5 flex flex-wrap items-center gap-x-6 gap-y-1 text-xs font-mono">
      {hasVerified && (
        <>
          <span className="text-term-faint uppercase tracking-wide">Skuteczność wygenerowanych taktyk:</span>
          <span className="text-term-dim">
            24h: <span className="text-term-amber">{hitRatePct(record.verified_24h_hits, record.verified_24h_total)}</span>
          </span>
          <span className="text-term-dim">
            7 dni: <span className="text-term-amber">{hitRatePct(record.verified_7d_hits, record.verified_7d_total)}</span>
          </span>
        </>
      )}
      {skipped > 0 && (
        <span className="text-term-faint">
          Pominięto {skipped} {tacticsPlural(skipped)} z powodu nieprawidłowej ceny odniesienia (zapis sprzed naprawy
          walidacji cen) — takie wpisy nie są punktowane.
        </span>
      )}
    </div>
  );
}
