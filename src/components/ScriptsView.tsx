import type { MarketContext } from "../types";
import { PineScriptSection } from "./PineScriptSection";
import { Panel } from "./Panel";

/**
 * Scripts tied to the market as a whole (index correlation, GSR) rather than to
 * the focused instrument, hence their own tab: they do not change when the
 * instrument changes. The per-instrument signal script stays in the overview,
 * where it is contextual to that instrument's analysis.
 */
export function ScriptsView({ marketContext }: { marketContext: MarketContext | null }) {
  if (!marketContext) {
    return (
      <Panel title="Skrypty TradingView">
        <p className="text-xs text-term-faint">
          Skrypty pojawią się po pobraniu danych rynkowych. Wróć do Przeglądu i kliknij
          "Odśwież dane rynkowe".
        </p>
      </Panel>
    );
  }

  // The backend emits the correlation script and its explanation together or
  // leaves out both, so one nullable value stands for the whole section: a
  // second check would imply a half-present state the backend cannot produce.
  const correlationScript =
    marketContext.pine_script_correlation === null
      ? null
      : {
          code: marketContext.pine_script_correlation,
          explanation: marketContext.pine_script_correlation_explanation!,
        };

  return (
    <div className="space-y-3">
      <Panel title="Skrypty TradingView">
        <p className="text-xs text-term-dim">
          Gotowe do wklejenia wskaźniki Pine Script v6 z opisem po polsku. Poniższe dotyczą
          całego rynku - skrypt sygnału dla konkretnego instrumentu znajdziesz w Przeglądzie,
          po wygenerowaniu jego analizy.
        </p>
      </Panel>

      {correlationScript ? (
        <PineScriptSection
          title="Pine Script: Korelacja indeksów"
          explanation={correlationScript.explanation}
          code={correlationScript.code}
        />
      ) : (
        <Panel title="Pine Script: Korelacja indeksów">
          <p className="text-[11px] text-term-amber">
            Korelacji nie zmierzono — żadna para indeksów nie miała dość wspólnych sesji w oknie
            90 dni, więc skrypt korelacji nie powstał. To stan danych, nie błąd aplikacji;
            pozostałe dane rynkowe i skrypt GSR są aktualne.
          </p>
        </Panel>
      )}
      <PineScriptSection
        title="Pine Script: Gold/Silver Ratio"
        explanation={marketContext.pine_script_gsr_explanation}
        code={marketContext.pine_script_gsr}
      />
    </div>
  );
}
