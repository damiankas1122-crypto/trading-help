import { useState } from "react";
import { INSTRUMENTS, INSTRUMENT_ICONS } from "../constants";

// Currently filters four fixed instruments, but as a real typeahead rather than
// a cycling switch, so a configurable watchlist can drop in unchanged.
export function InstrumentSearch({ value, onSelect }: { value: string; onSelect: (instrument: string) => void }) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [highlighted, setHighlighted] = useState(0);

  const matches = INSTRUMENTS.filter((i) => i.toLowerCase().includes(query.toLowerCase()));

  const choose = (instrument: string) => {
    onSelect(instrument);
    setQuery("");
    setOpen(false);
    setHighlighted(0);
  };

  // A terminal-styled field has to be operable without a mouse.
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!open) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlighted((h) => (matches.length === 0 ? 0 : (h + 1) % matches.length));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlighted((h) => (matches.length === 0 ? 0 : (h - 1 + matches.length) % matches.length));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const picked = matches[highlighted] ?? matches[0];
      if (picked) choose(picked);
    } else if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
      e.currentTarget.blur();
    }
  };

  return (
    <div className="relative flex-1 max-w-md">
      <div className="flex items-center gap-2 bg-black border border-term-line-strong px-3 py-1.5">
        <span className="text-term-amber">›</span>
        <input
          value={open ? query : value}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
            setHighlighted(0);
          }}
          onFocus={() => {
            setQuery("");
            setOpen(true);
            setHighlighted(0);
          }}
          onBlur={() => setTimeout(() => setOpen(false), 120)}
          onKeyDown={handleKeyDown}
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
          {matches.map((instrument, index) => (
            <button
              key={instrument}
              onMouseDown={() => choose(instrument)}
              onMouseEnter={() => setHighlighted(index)}
              className={`w-full flex items-center gap-2 px-3 py-2 text-left text-sm font-mono transition-colors ${
                index === highlighted
                  ? "bg-term-amber/10 text-term-amber"
                  : "text-term-dim hover:bg-term-amber/10 hover:text-term-amber"
              }`}
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
