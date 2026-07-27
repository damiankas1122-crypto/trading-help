import { useState } from "react";
import { Copy, Check } from "lucide-react";
import { copyToClipboard } from "../utils/clipboard";
import { Panel } from "./Panel";

export function PineScriptSection({
  title,
  explanation,
  code,
}: {
  title: string;
  explanation: string;
  code: string;
}) {
  const [copied, setCopied] = useState(false);

  const copyCode = async () => {
    if (await copyToClipboard(code)) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <Panel title={title} bodyClassName="space-y-3">
      <p className="text-xs text-term-dim font-mono whitespace-pre-wrap leading-relaxed">
        {explanation}
      </p>
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-term-faint uppercase tracking-wide">Pine Script v6</span>
        <button
          onClick={copyCode}
          className="flex items-center gap-1 text-xs text-term-dim hover:text-term-amber transition-colors"
        >
          {copied ? <Check size={14} /> : <Copy size={14} />}
          {copied ? "Skopiowano" : "Kopiuj"}
        </button>
      </div>
      <pre className="bg-black border border-term-line p-3 text-xs font-mono text-term-green overflow-x-auto whitespace-pre-wrap">
        {code}
      </pre>
    </Panel>
  );
}
