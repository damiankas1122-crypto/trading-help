import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import type {
  InstrumentInfo,
  LiveQuote,
  MarketContext,
  Snapshot,
  ViewId,
  TradingTactic,
  InstrumentBriefing,
} from "./types";
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

// Quotes are one light request per instrument, so a short cadence is cheap;
// the analytical context (candles, correlations, snapshot write) is heavier and
// its backend candle cache has a 5-minute TTL, so refreshing it faster than
// that would only re-serve the cache.
const LIVE_QUOTES_INTERVAL_MS = 60_000;
const MARKET_CONTEXT_REFRESH_MS = 5 * 60_000;

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
  const [liveQuotes, setLiveQuotes] = useState<Record<string, LiveQuote>>({});
  const [instrumentInfo, setInstrumentInfo] = useState<Record<string, InstrumentInfo>>({});
  // Whether the first quote round has finished, whatever its outcome. The tape
  // may not call anything "archival" before this: at startup the snapshot loads
  // from disk long before the network answers, so the marker used to flash for
  // a second on every healthy start - a warning nobody can read and that cries
  // wolf. After the first round it means what it says: we asked and got nothing.
  const [quotesSettled, setQuotesSettled] = useState(false);
  // Ref, not state: the interval callback closes over a stale render, so an
  // in-flight guard held in state would not be visible to it (CR-15).
  const marketContextInFlight = useRef(false);

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

  // Static catalogue: fetched once, never refreshed. Says what each instrument
  // actually is, so a futures price is not read as a spot price.
  useEffect(() => {
    invoke<InstrumentInfo[]>("get_instrument_catalog")
      .then((entries) =>
        setInstrumentInfo(Object.fromEntries(entries.map((entry) => [entry.id, entry])))
      )
      .catch((err) => console.warn("Instrument catalogue unavailable:", err));
  }, []);

  // Market data comes from Yahoo Finance with no rate limit, so it refreshes
  // automatically and independently of the AI briefing.
  const refreshMarketContext = async () => {
    if (marketContextInFlight.current) return;
    marketContextInFlight.current = true;
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
      marketContextInFlight.current = false;
      setMarketContextRefreshing(false);
    }
  };

  useEffect(() => {
    refreshMarketContext();
    const interval = setInterval(refreshMarketContext, MARKET_CONTEXT_REFRESH_MS);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Live quotes on their own short cadence. A failed round keeps the previous
  // values on screen; their visible "as of" time is what explains the gap.
  useEffect(() => {
    let cancelled = false;
    const tick = () => {
      invoke<LiveQuote[]>("get_live_quotes", { instruments: INSTRUMENTS })
        .then((quotes) => {
          if (cancelled || quotes.length === 0) return;
          setLiveQuotes((prev) => {
            const next = { ...prev };
            for (const quote of quotes) next[quote.instrument] = quote;
            return next;
          });
        })
        .catch((err) => console.warn("Live quotes failed:", err))
        .finally(() => {
          if (!cancelled) setQuotesSettled(true);
        });
    };
    tick();
    const interval = setInterval(tick, LIVE_QUOTES_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
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
        liveQuotes={liveQuotes}
        instrumentInfo={instrumentInfo}
        quotesSettled={quotesSettled}
        archivalTimestamp={!marketContext && lastSnapshot ? lastSnapshot.timestamp : null}
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
              instrumentInfo={instrumentInfo[focusedInstrument] ?? null}
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
