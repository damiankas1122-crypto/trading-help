import { Check, Download, RefreshCw } from "lucide-react";
import type { UpdateStatus } from "../types";

export function UpdateBanner({
  status,
  progress,
  version,
  errorMsg,
  onUpdate,
}: {
  status: UpdateStatus;
  progress: number;
  version: string | null;
  errorMsg: string | null;
  onUpdate: () => void;
}) {
  if (status === "idle") return null;

  return (
    <div className="bg-term-cyan/10 border border-term-cyan/40 px-4 py-2.5 flex items-center justify-between flex-wrap gap-2">
      <div className="flex items-center gap-2 text-term-cyan text-xs font-mono">
        {status === "available" && (
          <>
            <Download size={14} />
            <span>Dostępna nowa wersja{version ? ` (${version})` : ""}.</span>
          </>
        )}
        {status === "downloading" && (
          <>
            <RefreshCw size={14} className="animate-spin" />
            <span>Pobieranie aktualizacji... {progress}%</span>
          </>
        )}
        {status === "ready" && (
          <>
            <Check size={14} />
            <span>Zainstalowano, uruchamiam ponownie...</span>
          </>
        )}
        {status === "error" && (
          <span className="text-term-red">
            Nie udało się zaktualizować{errorMsg ? `: ${errorMsg}` : ""}. Spróbuj ponownie później lub pobierz instalator ręcznie.
          </span>
        )}
      </div>
      {status === "available" && (
        <button
          onClick={onUpdate}
          className="px-3 py-1.5 bg-term-cyan/10 border border-term-cyan text-term-cyan text-xs font-bold uppercase hover:bg-term-cyan/20 transition-colors"
        >
          Zaktualizuj teraz
        </button>
      )}
    </div>
  );
}
