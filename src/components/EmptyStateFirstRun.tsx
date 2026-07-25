export function EmptyStateFirstRun() {
  return (
    <div className="text-center py-12 space-y-2">
      <p className="text-slate-300 text-base font-semibold">Brak wcześniejszych analiz</p>
      <p className="text-slate-500 text-sm font-mono">
        Wybierz porę dnia powyżej, żeby wygenerować pierwszą analizę.
      </p>
    </div>
  );
}
