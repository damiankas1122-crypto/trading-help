export function signedPct(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

export function hitRatePct(hits: number, total: number): string {
  return total === 0 ? "brak danych" : `${Math.round((hits / total) * 100)}% (n=${total})`;
}

/**
 * Date + time for every data-age label. Deliberately no time-only variant: a
 * bare "09:12" hides the age of anything older than today, which is exactly
 * how a stale snapshot passed for live data.
 */
export function formatUnixDateTime(unixSeconds: string | number): string {
  const n = Number(unixSeconds);
  if (!Number.isFinite(n)) return "?";
  return new Date(n * 1000).toLocaleString("pl-PL", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Backend sends null when the correlation was not measured (too few shared sessions). */
export function formatCorrelation(value: number | null): string {
  return value === null ? "—" : value.toFixed(3);
}

const MAX_ERROR_LENGTH = 300;

/**
 * Tauri command errors are normally short readable sentences (thiserror
 * #[error(...)]). This is a safety net in case something long or technical
 * leaks through.
 */
export function formatErrorMessage(err: unknown): string {
  const text = String(err);
  return text.length > MAX_ERROR_LENGTH ? `${text.slice(0, MAX_ERROR_LENGTH)}…` : text;
}
