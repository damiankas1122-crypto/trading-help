import { useState } from "react";
import { Copy, Check } from "lucide-react";
import type { Citation } from "../types";
import { copyToClipboard } from "../utils/clipboard";
import { Panel } from "./Panel";

export function CitationsSection({ citations }: { citations: Citation[] }) {
  // Keyed by claim+label rather than index: the list reloads on every analysis,
  // and an index would tie the "copied" state to a position instead of a citation.
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  if (citations.length === 0) return null;

  const citationKey = (c: Citation) => `${c.claim}|${c.evidence_label}`;

  const copyLink = async (link: string, key: string) => {
    if (await copyToClipboard(link)) {
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 2000);
    }
  };

  return (
    <Panel title="Cytowania">
      <ul className="space-y-2">
        {citations.map((c) => {
          const key = citationKey(c);
          return (
          <li key={key} className="text-xs font-mono border border-term-line p-2.5 space-y-1.5">
            <p className="text-term-dim italic">&ldquo;{c.claim}&rdquo;</p>
            <div className="flex items-center justify-between gap-2">
              <span className="text-term-text">
                <span className="text-term-faint uppercase mr-1">
                  {c.evidence_type === "news" ? "News:" : "Dane:"}
                </span>
                {c.evidence_label}
              </span>
              {c.evidence_link && (
                <button
                  onClick={() => copyLink(c.evidence_link as string, key)}
                  className="flex items-center gap-1 text-term-faint hover:text-term-cyan shrink-0 transition-colors"
                >
                  {copiedKey === key ? <Check size={12} /> : <Copy size={12} />}
                  {copiedKey === key ? "Skopiowano" : "Kopiuj link"}
                </button>
              )}
            </div>
          </li>
          );
        })}
      </ul>
    </Panel>
  );
}
