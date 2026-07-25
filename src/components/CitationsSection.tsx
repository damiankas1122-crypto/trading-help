import { useState } from "react";
import { Copy, Check } from "lucide-react";
import type { Citation } from "../types";
import { copyToClipboard } from "../utils/clipboard";

export function CitationsSection({ citations }: { citations: Citation[] }) {
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  if (citations.length === 0) return null;

  const copyLink = async (link: string, index: number) => {
    if (await copyToClipboard(link)) {
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 2000);
    }
  };

  return (
    <div className="space-y-2">
      <h3 className="text-cyan-400 text-xs font-bold uppercase tracking-[0.15em]">Źródła</h3>
      <ul className="space-y-2">
        {citations.map((c, i) => (
          <li key={i} className="text-xs font-mono bg-black/30 rounded-lg p-3 space-y-1.5">
            <p className="text-slate-400 italic">&ldquo;{c.claim}&rdquo;</p>
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-300">
                <span className="text-slate-600 uppercase mr-1">
                  {c.evidence_type === "news" ? "News:" : "Dane:"}
                </span>
                {c.evidence_label}
              </span>
              {c.evidence_link && (
                <button
                  onClick={() => copyLink(c.evidence_link as string, i)}
                  className="flex items-center gap-1 text-slate-500 hover:text-cyan-300 shrink-0"
                >
                  {copiedIndex === i ? <Check size={12} /> : <Copy size={12} />}
                  {copiedIndex === i ? "Skopiowano" : "Kopiuj link"}
                </button>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
