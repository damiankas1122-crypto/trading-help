import { VIEW_NAV_ITEMS } from "../constants";
import type { ViewId } from "../types";

export function ViewNav({ active, onChange }: { active: ViewId; onChange: (view: ViewId) => void }) {
  return (
    <nav className="border-r border-term-line bg-term-panel p-2.5 flex flex-col gap-1 w-40 shrink-0 font-mono">
      {VIEW_NAV_ITEMS.map((item) => {
        const isActive = item.view === active;
        const disabled = item.view === null;
        return (
          <button
            key={item.label}
            disabled={disabled}
            onClick={() => item.view && onChange(item.view)}
            title={disabled ? "Wkrótce" : undefined}
            className={`flex items-center justify-between border px-2.5 py-2 text-[11.5px] tracking-wide transition-colors ${
              disabled
                ? "border-term-line text-term-faint/50 cursor-default"
                : isActive
                ? "border-term-amber text-term-amber bg-term-amber/10"
                : "border-term-line text-term-dim hover:border-term-line-strong hover:text-term-text"
            }`}
          >
            <span>{item.label}</span>
            <span className={disabled ? "text-term-faint/50" : isActive ? "text-term-amber" : "text-term-faint"}>
              {disabled ? "•" : item.fKey}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
