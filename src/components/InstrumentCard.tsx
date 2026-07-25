import { useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { InstrumentBriefing } from "../types";
import { INSTRUMENT_ICONS } from "../constants";
import { PineScriptSection } from "./PineScriptSection";
import { CitationsSection } from "./CitationsSection";
import { TradingTacticSection } from "./TradingTacticSection";

export function InstrumentCard({ briefing }: { briefing: InstrumentBriefing }) {
  const [expanded, setExpanded] = useState(true);

  return (
    <div className="bg-[#0a0a1a]/60 rounded-2xl border border-cyan-900/30 overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between px-5 py-4 hover:bg-cyan-500/5 transition-colors"
      >
        <div className="flex items-center gap-2 text-cyan-400 font-bold text-sm tracking-[0.15em] uppercase">
          {INSTRUMENT_ICONS[briefing.instrument]}
          {briefing.instrument}
        </div>
        {expanded ? <ChevronUp size={16} className="text-slate-500" /> : <ChevronDown size={16} className="text-slate-500" />}
      </button>
      {expanded && (
        <div className="px-5 pb-5 space-y-4">
          <div className="text-sm text-slate-300 font-mono whitespace-pre-wrap leading-relaxed">
            {briefing.commentary}
          </div>
          <CitationsSection citations={briefing.citations} />
          <PineScriptSection
            title={`Pine Script: Sygnał ${briefing.instrument}`}
            explanation={briefing.pine_script_signal_explanation}
            code={briefing.pine_script_signal}
          />
          <TradingTacticSection instrument={briefing.instrument} />
        </div>
      )}
    </div>
  );
}
