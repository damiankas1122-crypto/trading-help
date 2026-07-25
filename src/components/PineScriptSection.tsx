import { useState } from "react";
import { Copy, Check } from "lucide-react";
import { copyToClipboard } from "../utils/clipboard";

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
    <div className="bg-[#0a0a1a]/60 rounded-2xl border border-cyan-900/30 p-5 space-y-3">
      <h3 className="text-cyan-400 text-xs font-bold uppercase tracking-[0.15em]">{title}</h3>
      <p className="text-sm text-slate-300 font-mono whitespace-pre-wrap leading-relaxed">
        {explanation}
      </p>
      <div className="flex items-center justify-between">
        <span className="text-xs text-slate-500 uppercase tracking-wide">Pine Script v6</span>
        <button
          onClick={copyCode}
          className="flex items-center gap-1 text-xs text-slate-400 hover:text-cyan-300"
        >
          {copied ? <Check size={14} /> : <Copy size={14} />}
          {copied ? "Skopiowano" : "Kopiuj"}
        </button>
      </div>
      <pre className="bg-black/40 rounded-lg p-3 text-xs font-mono text-green-400 overflow-x-auto whitespace-pre-wrap">
        {code}
      </pre>
    </div>
  );
}
