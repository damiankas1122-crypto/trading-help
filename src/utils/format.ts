export function signedPct(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

export function hitRatePct(hits: number, total: number): string {
  return total === 0 ? "brak danych" : `${Math.round((hits / total) * 100)}% (n=${total})`;
}

/** Backend timestamps arrive as unix seconds in a string (OffsetDateTime::unix_timestamp). */
export function formatUnixTimestamp(unixSeconds: string): string {
  const n = Number(unixSeconds);
  if (!Number.isFinite(n)) return "?";
  return new Date(n * 1000).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
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
