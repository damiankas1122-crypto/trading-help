import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sunrise, Sun, Moon } from "lucide-react";
import ThreeBackground from "./ThreeBackground";
import "./App.css";
import type { BriefingProgress, FullBriefing, Snapshot } from "./types";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { PineScriptSection } from "./components/PineScriptSection";
import { InstrumentCard } from "./components/InstrumentCard";
import { EmptyStateFirstRun } from "./components/EmptyStateFirstRun";
import { DisclaimerFooter } from "./components/DisclaimerFooter";
import { ApiKeyOnboarding } from "./components/ApiKeyOnboarding";
import { UpdateBanner } from "./components/UpdateBanner";
import { TacticTrackRecordBanner } from "./components/TacticTrackRecordBanner";
import { LastSnapshotPreview } from "./components/LastSnapshotPreview";

function App() {
  const updater = useAppUpdater();
  const [briefing, setBriefing] = useState<FullBriefing | null>(null);
  const [lastSnapshot, setLastSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<string | null>(null);
  const [hasApiKey, setHasApiKey] = useState<boolean | null>(null);
  const [progress, setProgress] = useState<BriefingProgress | null>(null);

   useEffect(() => {
    const unlisten = listen<BriefingProgress>("briefing-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    invoke<Snapshot | null>("get_last_snapshot")
      .then(setLastSnapshot)
      .catch(() => setLastSnapshot(null));
  }, []);

   useEffect(() => {
    invoke<boolean>("has_gemini_api_key")
      .then(setHasApiKey)
      .catch(() => setHasApiKey(false));
  }, []);

  const runFullBriefing = async (slot: string) => {
    setLoading(true);
    setError(null);
    setProgress(null);   
    try {
      const result = await invoke<FullBriefing>("get_full_briefing", { slot });
      setBriefing(result);
      setLastUpdated(new Date().toLocaleString("pl-PL"));
    } catch (err) {
      console.error("Błąd briefingu:", err);
      setError(String(err));
    } finally {
      setLoading(false);
      setProgress(null);
    }
  };

  if (hasApiKey === null) {
    return <div className="h-screen w-screen bg-[#05050a]" />;
  }

  if (hasApiKey === false) {
    return <ApiKeyOnboarding onSaved={() => setHasApiKey(true)} />;
  }
  
  return (
    <div className="h-screen bg-[#05050a] text-white relative flex flex-col overflow-hidden">
      <ThreeBackground />

     <header className="h-16 border-b border-cyan-900/50 flex items-center justify-between px-6 bg-[#0a0a1a]/70 backdrop-blur-sm sticky top-0 z-10">
        <h1 className="text-xl font-black text-cyan-400 tracking-[0.2em]">TRADING HELP</h1>
        <div className="flex items-center gap-4">
          {lastUpdated && (
            <span className="text-xs text-slate-500 font-mono">Ostatnia aktualizacja: {lastUpdated}</span>
          )}
          <button
            onClick={async () => {
              if (!confirm("Usunąć zapisany klucz API i wprowadzić nowy?")) return;
              try {
                await invoke("delete_gemini_api_key");
                setHasApiKey(false);
              } catch (err) {
                console.error("Błąd usuwania klucza:", err);
              }
            }}
            className="text-xs text-slate-500 hover:text-cyan-400 font-mono underline underline-offset-2"
          >
            Zmień klucz API
          </button>
        </div>
      </header>
      <main className="max-w-4xl mx-auto p-6 space-y-6 relative z-[1] flex-1 w-full overflow-y-auto">
        <UpdateBanner
          status={updater.status}
          progress={updater.progress}
          version={updater.version}
          errorMsg={updater.errorMsg}
          onUpdate={updater.downloadAndInstall}
        />

        <TacticTrackRecordBanner />

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
            <p className="text-cyan-400 text-xs font-mono mt-3 animate-pulse">
              {progress
                ? `Analizuję ${progress.instrument}... (${progress.step}/${progress.total})`
                : "Przygotowuję dane rynkowe..."}
            </p>
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

            {briefing.is_stale_data && (
              <div className="bg-amber-950/30 border border-amber-700/50 rounded-2xl p-4 text-amber-300 text-xs font-mono">
                {briefing.stale_data_message}
              </div>
            )}

            {!briefing.is_stale_data && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {briefing.instrument_briefings.map((b) => (
                  <InstrumentCard key={b.instrument} briefing={b} />
                ))}
              </div>
            )}

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

            {!briefing.is_stale_data && (
              <>
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
          </>
        )}

        {!briefing && !loading && !error && (
          lastSnapshot
            ? <LastSnapshotPreview snapshot={lastSnapshot} onRefresh={() => runFullBriefing(lastSnapshot.slot)} />
            : <EmptyStateFirstRun />
        )}
      </main>

      <DisclaimerFooter />
    </div>
  );
}

export default App;
