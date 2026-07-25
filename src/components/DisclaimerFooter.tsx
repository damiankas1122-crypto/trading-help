import { useState } from "react";

export function DisclaimerFooter() {
  const [dismissed, setDismissed] = useState(false);
  if (dismissed) return null;

  return (
    <footer className="shrink-0 border-t border-cyan-900/50 flex items-center justify-center gap-4 px-6 py-2 bg-[#0a0a1a]/90 backdrop-blur-sm z-10">
      <p className="text-[11px] text-slate-500 font-mono text-center">
        Treści generowane przez AI mają charakter wyłącznie informacyjno-edukacyjny i{" "}
        <span className="text-slate-400 font-semibold">nie stanowią porady inwestycyjnej</span>{" "}
        ani rekomendacji w rozumieniu przepisów prawa. Decyzje inwestycyjne podejmujesz na własną odpowiedzialność.
      </p>
      <button
        onClick={() => setDismissed(true)}
        className="shrink-0 px-3 py-1 rounded bg-cyan-500/10 border border-cyan-700/50 text-cyan-400 text-[10px] font-bold uppercase tracking-wider hover:bg-cyan-500/20 transition-colors"
      >
        Rozumiem
      </button>
    </footer>
  );
}
