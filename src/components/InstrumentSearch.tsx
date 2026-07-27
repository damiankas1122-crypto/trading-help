import { useState } from "react";
import { INSTRUMENTS, INSTRUMENT_ICONS } from "../constants";

// dziś filtruje 4 stałe instrumenty, ale jako prawdziwy typeahead - gotowe
// pod konfigurowalną watchlistę (Etap 6), nie przełącznik cykliczny
export function InstrumentSearch({ value, onSelect }: { value: string; onSelect: (instrument: string) => void }) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);

  const matches = INSTRUMENTS.filter((i) => i.toLowerCase().includes(query.toLowerCase()));

  return (
    <div className="relative flex-1 max-w-md">
      <div className="flex items-center gap-2 bg-black border border-term-line-strong px-3 py-1.5">
        <span className="text-term-amber">›</span>
        <input
          value={open ? query : value}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
          }}
          onFocus={() => {
            setQuery("");
            setOpen(true);
          }}
          onBlur={() => setTimeout(() => setOpen(false), 120)}
          placeholder="Szukaj instrumentu..."
          className="flex-1 bg-transparent outline-none text-term-text font-mono text-sm min-w-0"
        />
        <span className="text-term-faint text-[10px] shrink-0">&lt;GO&gt;</span>
      </div>
      {open && (
        <div className="absolute top-full left-0 right-0 bg-term-panel border border-term-line-strong border-t-0 z-20">
          {matches.length === 0 && (
            <div className="px-3 py-2 text-term-faint text-xs font-mono">Brak instrumentu</div>
          )}
          {matches.map((instrument) => (
            <button
              key={instrument}
              onMouseDown={() => {
                onSelect(instrument);
                setQuery("");
                setOpen(false);
              }}
              className="w-full flex items-center gap-2 px-3 py-2 text-left text-sm font-mono text-term-dim hover:bg-term-amber/10 hover:text-term-amber transition-colors"
            >
              <span className="text-term-faint">{INSTRUMENT_ICONS[instrument]}</span>
              {instrument}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
