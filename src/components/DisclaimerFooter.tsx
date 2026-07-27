import { useState } from "react";

export function DisclaimerFooter() {
  const [dismissed, setDismissed] = useState(false);
  if (dismissed) return null;

  return (
    <footer className="shrink-0 border-t border-term-line flex items-center justify-center gap-4 px-6 py-2 bg-black z-10">
      <p className="text-[11px] text-term-faint font-mono text-center">
        Treści generowane przez AI mają charakter wyłącznie informacyjno-edukacyjny i{" "}
        <span className="text-term-dim font-semibold">nie stanowią porady inwestycyjnej</span>{" "}
        ani rekomendacji w rozumieniu przepisów prawa. Decyzje inwestycyjne podejmujesz na własną odpowiedzialność.
      </p>
      <button
        onClick={() => setDismissed(true)}
        className="shrink-0 px-3 py-1 border border-term-line-strong text-term-amber text-[10px] font-bold uppercase tracking-wider hover:bg-term-amber/10 transition-colors"
      >
        Rozumiem
      </button>
    </footer>
  );
}
