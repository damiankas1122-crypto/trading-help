import type { TradingTactic } from "../types";
import { TradingTacticSection } from "./TradingTacticSection";
import { TacticTrackRecordBanner } from "./TacticTrackRecordBanner";
import { Panel } from "./Panel";

export function TacticsView({
  instrument,
  tactic,
  onTacticChange,
}: {
  instrument: string;
  tactic: TradingTactic | null;
  onTacticChange: (tactic: TradingTactic) => void;
}) {
  return (
    <div className="space-y-3">
      <TacticTrackRecordBanner />
      <Panel title={`Taktyka // ${instrument}`}>
        <TradingTacticSection instrument={instrument} tactic={tactic} onTacticChange={onTacticChange} />
      </Panel>
    </div>
  );
}
