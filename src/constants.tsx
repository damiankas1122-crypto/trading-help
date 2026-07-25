import { TrendingUp, BarChart3, Coins, Gem } from "lucide-react";

export const INSTRUMENT_ICONS: Record<string, React.ReactNode> = {
  NASDAQ: <TrendingUp size={18} />,
  SP500: <BarChart3 size={18} />,
  GOLD: <Coins size={18} />,
  SILVER: <Gem size={18} />,
};

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
