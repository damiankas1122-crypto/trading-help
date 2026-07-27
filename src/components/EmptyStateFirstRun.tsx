export function EmptyStateFirstRun() {
  return (
    <div className="text-center py-12 space-y-2 font-mono">
      <p className="text-term-text text-base font-semibold">Brak wcześniejszych analiz</p>
      <p className="text-term-dim text-sm">
        Wybierz instrument w wyszukiwarce powyżej i kliknij "Analizuj", żeby wygenerować pierwszy briefing.
      </p>
    </div>
  );
}
