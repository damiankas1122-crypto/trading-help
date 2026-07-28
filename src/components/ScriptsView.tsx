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

  return (
    <div className="space-y-3">
      <Panel title="Skrypty TradingView">
        <p className="text-xs text-term-dim">
          Gotowe do wklejenia wskaźniki Pine Script v6 z opisem po polsku. Poniższe dotyczą
          całego rynku - skrypt sygnału dla konkretnego instrumentu znajdziesz w Przeglądzie,
          po wygenerowaniu jego analizy.
        </p>
      </Panel>

      <PineScriptSection
        title="Pine Script: Korelacja indeksów"
        explanation={marketContext.pine_script_correlation_explanation}
        code={marketContext.pine_script_correlation}
      />
      <PineScriptSection
        title="Pine Script: Gold/Silver Ratio"
        explanation={marketContext.pine_script_gsr_explanation}
        code={marketContext.pine_script_gsr}
      />
    </div>
  );
}
