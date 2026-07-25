export function signedPct(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

export function hitRatePct(hits: number, total: number): string {
  return total === 0 ? "brak danych" : `${Math.round((hits / total) * 100)}% (n=${total})`;
}
