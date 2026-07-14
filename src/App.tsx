import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { TrendingUp, BarChart3, Coins, Gem, Copy, Check, ChevronDown, ChevronUp, Sunrise, Sun, Moon } from "lucide-react";
import ThreeBackground from "./ThreeBackground";
import "./App.css";

type AnalyticalReport = {
  symbol: string;
  correlation: number;
  volatility: number;
  sentiment_impact: number;
  timestamp: string;
};

type PreciousMetalsReport = {
  correlation: number;
  current_gsr: number;
  gsr_30d_ago: number;
  gsr_change_pct: number;
  gold_volatility: number;
  silver_volatility: number;
  timestamp: string;
};

type InstrumentBriefing = {
  instrument: string;
  commentary: string;
};

type FullBriefing = {
  slot: string;
  compared_to: string | null;
  equity_reports: AnalyticalReport[];
  metals_report: PreciousMetalsReport;
  instrument_briefings: InstrumentBriefing[];
  pine_script_correlation: string;
  pine_script_correlation_explanation: string;
  pine_script_gsr: string;
  pine_script_gsr_explanation: string;
};

type Snapshot = {
  equity_reports: AnalyticalReport[];
  metals_report: PreciousMetalsReport;
  timestamp: string;
  slot: string;
};

const INSTRUMENT_ICONS: Record<string, React.ReactNode> = {
  NASDAQ: <TrendingUp size={18} />,
  SP500: <BarChart3 size={18} />,
  GOLD: <Coins size={18} />,
  SILVER: <Gem size={18} />,
};

function InstrumentCard({ briefing }: { briefing: InstrumentBriefing }) {
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
        <div className="px-5 pb-5 text-sm text-slate-300 font-mono whitespace-pre-wrap leading-relaxed">
          {briefing.commentary}
        </div>
      )}
    </div>
  );
}

function PineScriptSection({
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
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard niedostępny (np. brak uprawnień) - po prostu nie pokazujemy potwierdzenia
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

// Wypełnia pusty stan realną treścią: ostatni zapisany odczyt (jeśli istnieje)
// zamiast pustej przestrzeni z pojedynczym zdaniem.
function LastSnapshotPreview({ snapshot, onRefresh }: { snapshot: Snapshot; onRefresh: () => void }) {
  return (
    <div className="bg-[#0a0a1a]/40 rounded-2xl border border-cyan-900/20 p-6 opacity-80">
      <div className="flex items-center justify-between mb-4 flex-wrap gap-2">
        <span className="text-slate-400 text-xs font-bold uppercase tracking-[0.15em]">
          Ostatnia zapisana analiza — {snapshot.slot}
        </span>
        <button
          onClick={onRefresh}
          className="text-xs text-cyan-400 hover:text-cyan-300 font-mono underline underline-offset-2"
        >
          Odśwież teraz
        </button>
      </div>
      <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-xs font-mono text-slate-500">
        {snapshot.equity_reports.map((r) => (
          <div key={r.symbol}>
            <span>{r.symbol}: </span>
            <span className="text-slate-400">{r.correlation.toFixed(3)}</span>
          </div>
        ))}
        <div>
          <span>GSR: </span>
          <span className="text-slate-400">{snapshot.metals_report.current_gsr.toFixed(2)}</span>
        </div>
      </div>
      <p className="text-slate-600 text-xs font-mono mt-4">
        Wybierz porę dnia powyżej, żeby wygenerować świeżą analizę z aktualnymi komentarzami.
      </p>
    </div>
  );
}

function EmptyStateFirstRun() {
  return (
    <div className="text-center py-12 space-y-2">
      <p className="text-slate-300 text-base font-semibold">Brak wcześniejszych analiz</p>
      <p className="text-slate-500 text-sm font-mono">
        Wybierz porę dnia powyżej, żeby wygenerować pierwszą analizę.
      </p>
    </div>
  );
}

function App() {
  const [briefing, setBriefing] = useState<FullBriefing | null>(null);
  const [lastSnapshot, setLastSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<string | null>(null);

  useEffect(() => {
    invoke<Snapshot | null>("get_last_snapshot")
      .then(setLastSnapshot)
      .catch(() => setLastSnapshot(null));
  }, []);

  const runFullBriefing = async (slot: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FullBriefing>("get_full_briefing", { slot });
      setBriefing(result);
      setLastUpdated(new Date().toLocaleString("pl-PL"));
    } catch (err) {
      console.error("Błąd briefingu:", err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-[#05050a] text-white relative">
      <ThreeBackground />

      <header className="h-16 border-b border-cyan-900/50 flex items-center justify-between px-6 bg-[#0a0a1a]/70 backdrop-blur-sm sticky top-0 z-10">
        <h1 className="text-xl font-black text-cyan-400 tracking-[0.2em]">TRADING_HELP</h1>
        {lastUpdated && (
          <span className="text-xs text-slate-500 font-mono">Ostatnia aktualizacja: {lastUpdated}</span>
        )}
      </header>

      <main className="max-w-4xl mx-auto p-6 space-y-6 relative z-[1]">
        <div className="bg-[#0a0a1a]/60 rounded-2xl border border-cyan-900/30 p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <p className="text-slate-300 font-mono text-sm mb-1">AI Co-Pilot Engine</p>
              <p className="text-slate-500 text-xs">
                Analiza NASDAQ, SP500, Gold i Silver + gotowe skrypty do TradingView.
              </p>
            </div>
          </div>
          <div className="flex gap-2 flex-wrap">
            <button
              onClick={() => runFullBriefing("PORANNA")}
              disabled={loading}
              className="flex items-center gap-2 px-4 py-2.5 rounded bg-cyan-500/20 border border-cyan-500 text-cyan-300 text-xs font-bold uppercase"
            >
              <Sunrise size={14} /> Analiza poranna
            </button>
            <button
              onClick={() => runFullBriefing("POPOŁUDNIOWA")}
              disabled={loading}
              className="flex items-center gap-2 px-4 py-2.5 rounded bg-cyan-500/20 border border-cyan-500 text-cyan-300 text-xs font-bold uppercase"
            >
              <Sun size={14} /> Analiza popołudniowa
            </button>
            <button
              onClick={() => runFullBriefing("WIECZORNA")}
              disabled={loading}
              className="flex items-center gap-2 px-4 py-2.5 rounded bg-cyan-500/20 border border-cyan-500 text-cyan-300 text-xs font-bold uppercase"
            >
              <Moon size={14} /> Analiza wieczorna
            </button>
          </div>
          {loading && (
            <p className="text-cyan-400 text-xs font-mono mt-3 animate-pulse">Analizuję rynki...</p>
          )}
        </div>

        {error && (
          <div className="bg-red-950/30 border border-red-900/50 rounded-2xl p-4 text-red-400 text-xs font-mono whitespace-pre-wrap">
            {error}
          </div>
        )}

        {briefing && (
          <>
            <div className="flex items-center justify-between flex-wrap gap-2">
              <span className="text-cyan-400 text-sm font-bold uppercase tracking-[0.15em]">
                Analiza {briefing.slot}
              </span>
              {briefing.compared_to && (
                <span className="text-xs text-slate-500 font-mono">
                  Porównano z poprzednią analizą: {briefing.compared_to}
                </span>
              )}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {briefing.instrument_briefings.map((b) => (
                <InstrumentCard key={b.instrument} briefing={b} />
              ))}
            </div>

            <div className="bg-[#0a0a1a]/60 rounded-2xl border border-cyan-900/30 p-5">
              <p className="text-cyan-400 text-xs font-bold uppercase tracking-[0.15em] mb-3">Surowe dane</p>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-xs font-mono text-slate-400">
                {briefing.equity_reports.map((r) => (
                  <div key={r.symbol}>
                    <span className="text-slate-500">{r.symbol}: </span>
                    <span className="text-slate-300">{r.correlation.toFixed(3)}</span>
                  </div>
                ))}
                <div>
                  <span className="text-slate-500">GSR: </span>
                  <span className="text-slate-300">{briefing.metals_report.current_gsr.toFixed(2)}</span>
                </div>
                <div>
                  <span className="text-slate-500">Au-Ag corr: </span>
                  <span className="text-slate-300">{briefing.metals_report.correlation.toFixed(3)}</span>
                </div>
              </div>
            </div>

            <PineScriptSection
              title="Pine Script: Korelacja indeksów"
              explanation={briefing.pine_script_correlation_explanation}
              code={briefing.pine_script_correlation}
            />

            <PineScriptSection
              title="Pine Script: Gold/Silver Ratio"
              explanation={briefing.pine_script_gsr_explanation}
              code={briefing.pine_script_gsr}
            />
          </>
        )}

        {!briefing && !loading && !error && (
          lastSnapshot
            ? <LastSnapshotPreview snapshot={lastSnapshot} onRefresh={() => runFullBriefing(lastSnapshot.slot)} />
            : <EmptyStateFirstRun />
        )}
      </main>
    </div>
  );
}

export default App;
