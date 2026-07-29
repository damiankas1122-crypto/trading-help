import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import type { MarketContext, Snapshot, ViewId, TradingTactic, InstrumentBriefing } from "./types";
import { INSTRUMENTS } from "./constants";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { formatErrorMessage } from "./utils/format";
import { cancelOperation, isCancellation, newOperationId } from "./utils/aiOperations";
import { TickerTape } from "./components/TickerTape";
import { ViewNav } from "./components/ViewNav";
import { InstrumentSearch } from "./components/InstrumentSearch";
import { OverviewView } from "./components/OverviewView";
import { TacticsView } from "./components/TacticsView";
import { CorrelationBuilderView } from "./components/CorrelationBuilderView";
import { ScriptsView } from "./components/ScriptsView";
import { SettingsView } from "./components/SettingsView";
import { DisclaimerFooter } from "./components/DisclaimerFooter";
import { ApiKeyOnboarding } from "./components/ApiKeyOnboarding";
import { UpdateBanner } from "./components/UpdateBanner";

function App() {
  const updater = useAppUpdater();
  const [marketContext, setMarketContext] = useState<MarketContext | null>(null);
  const [marketContextError, setMarketContextError] = useState<string | null>(null);
  const [marketContextRefreshing, setMarketContextRefreshing] = useState(false);
  const [lastSnapshot, setLastSnapshot] = useState<Snapshot | null>(null);
  const [hasApiKey, setHasApiKey] = useState<boolean | null>(null);
  const [activeView, setActiveView] = useState<ViewId>("przeglad");
  const [focusedInstrument, setFocusedInstrument] = useState<string>(INSTRUMENTS[0]);
  // Keyed per instrument so switching focus does not discard an existing tactic
  // or briefing, nor force a redundant Gemini call.
  const [tactics, setTactics] = useState<Record<string, TradingTactic | null>>({});
  const [instrumentBriefings, setInstrumentBriefings] = useState<Record<string, InstrumentBriefing | null>>({});
  const [briefingLoading, setBriefingLoading] = useState(false);
  const [briefingError, setBriefingError] = useState<string | null>(null);
  const [briefingOperationId, setBriefingOperationId] = useState<string | null>(null);

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

  // Market data comes from Yahoo Finance with no rate limit, so it refreshes
  // automatically and independently of the AI briefing.
  const refreshMarketContext = async () => {
    setMarketContextRefreshing(true);
    try {
      const result = await invoke<MarketContext>("get_market_context");
      setMarketContext(result);
      setMarketContextError(null);
    } catch (err) {
      console.error("Failed to fetch market data:", err);
      // marketContext is deliberately kept: stale data stays visible and
      // OverviewView renders an error banner alongside it.
      setMarketContextError(formatErrorMessage(err));
    } finally {
      setMarketContextRefreshing(false);
    }
  };

  useEffect(() => {
    refreshMarketContext();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const runInstrumentBriefing = async (instrument: string) => {
    const operationId = newOperationId();
    setBriefingOperationId(operationId);
    setBriefingLoading(true);
    setBriefingError(null);
    try {
      const result = await invoke<InstrumentBriefing>("get_instrument_briefing", {
        instrument,
        operationId,
      });
      setInstrumentBriefings((prev) => ({ ...prev, [instrument]: result }));
    } catch (err) {
      // Cancelling returns to idle: no error panel, nothing to explain.
      if (!isCancellation(err)) {
        console.error("Briefing failed:", err);
        setBriefingError(formatErrorMessage(err));
      }
    } finally {
      setBriefingLoading(false);
      setBriefingOperationId(null);
    }
  };

  if (hasApiKey === null) {
    return <div className="h-screen w-screen bg-term-bg" />;
  }

  if (hasApiKey === false) {
    return <ApiKeyOnboarding onSaved={() => setHasApiKey(true)} />;
  }

  return (
    <div className="h-screen bg-term-bg text-term-text flex flex-col overflow-hidden font-mono">
      <TickerTape
        equityReports={marketContext?.equity_reports ?? lastSnapshot?.equity_reports ?? null}
        metalsReport={marketContext?.metals_report ?? lastSnapshot?.metals_report ?? null}
      />

      <header className="h-14 shrink-0 border-b border-term-line flex items-center gap-4 px-4 bg-term-panel">
        <h1 className="text-term-amber font-black tracking-[0.15em] text-sm whitespace-nowrap">
          TRADING HELP <span className="text-term-faint font-normal">// TERMINAL</span>
        </h1>
        <InstrumentSearch value={focusedInstrument} onSelect={setFocusedInstrument} />
      </header>

      {updater.status !== "idle" && (
        <div className="px-4 pt-3 shrink-0">
          <UpdateBanner
            status={updater.status}
            progress={updater.progress}
            version={updater.version}
            errorMsg={updater.errorMsg}
            onUpdate={updater.downloadAndInstall}
          />
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <ViewNav active={activeView} onChange={setActiveView} />

        <main className="flex-1 overflow-y-auto p-4">
          {activeView === "przeglad" && (
            <OverviewView
              instrument={focusedInstrument}
              marketContext={marketContext}
              marketContextError={marketContextError}
              marketContextRefreshing={marketContextRefreshing}
              lastSnapshot={lastSnapshot}
              onRefreshMarketContext={refreshMarketContext}
              instrumentBriefing={instrumentBriefings[focusedInstrument] ?? null}
              briefingLoading={briefingLoading}
              briefingError={briefingError}
              onAnalyze={() => runInstrumentBriefing(focusedInstrument)}
              onCancelAnalysis={
                briefingOperationId ? () => cancelOperation(briefingOperationId) : undefined
              }
            />
          )}
          {activeView === "taktyka" && (
            <TacticsView
              instrument={focusedInstrument}
              tactic={tactics[focusedInstrument] ?? null}
              onTacticChange={(tactic) =>
                setTactics((prev) => ({ ...prev, [focusedInstrument]: tactic }))
              }
            />
          )}
          {activeView === "korelacje" && <CorrelationBuilderView />}
          {activeView === "skrypty" && <ScriptsView marketContext={marketContext} />}
          {activeView === "ustawienia" && <SettingsView onKeyDeleted={() => setHasApiKey(false)} />}
        </main>
      </div>

      <DisclaimerFooter />
    </div>
  );
}

export default App;
