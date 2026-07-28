import type { MarketContext } from "../types";
import { PineScriptSection } from "./PineScriptSection";
import { Panel } from "./Panel";

/**
 * Skrypty przypięte do CAŁEGO rynku, nie do instrumentu w fokusie (korelacja
 * indeksów, GSR) - dlatego własna zakładka, a nie Przegląd: nie zmieniają się
 * gdy przełączysz instrument. Skrypt sygnału per-instrument zostaje w
 * Przeglądzie, bo jest kontekstowy do analizy tego instrumentu.
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
