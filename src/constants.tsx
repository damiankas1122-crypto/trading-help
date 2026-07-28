import { TrendingUp, BarChart3, Coins, Gem } from "lucide-react";
import type { ViewId } from "./types";

export const INSTRUMENT_ICONS: Record<string, React.ReactNode> = {
  NASDAQ: <TrendingUp size={18} />,
  SP500: <BarChart3 size={18} />,
  GOLD: <Coins size={18} />,
  SILVER: <Gem size={18} />,
};

// Order drives both the ticker tape and instrument search results.
export const INSTRUMENTS = ["NASDAQ", "SP500", "GOLD", "SILVER"];

// view: null marks a tab that is not built yet (greyed out, "coming soon").
export const VIEW_NAV_ITEMS: { label: string; fKey: string; view: ViewId | null }[] = [
  { label: "Przegląd", fKey: "F1", view: "przeglad" },
  { label: "Taktyka", fKey: "F2", view: "taktyka" },
  { label: "Korelacje", fKey: "F3", view: "korelacje" },
  { label: "Skrypty", fKey: "F4", view: "skrypty" },
  { label: "Kalendarz", fKey: "F5", view: null },
  { label: "Heatmapa", fKey: "F6", view: null },
  { label: "Alerty", fKey: "F7", view: null },
  { label: "Ustawienia", fKey: "F8", view: "ustawienia" },
];

export const TACTIC_SCENARIO_STYLE: Record<string, string> = {
  bull: "text-green-400 border-green-900/50 bg-green-950/20",
  bear: "text-red-400 border-red-900/50 bg-red-950/20",
  neutral: "text-slate-400 border-slate-700/50 bg-slate-900/20",
};

export const TACTIC_SCENARIO_LABEL: Record<string, string> = {
  bull: "Wzrostowy (bull)",
  bear: "Spadkowy (bear)",
  neutral: "Neutralny",
};
