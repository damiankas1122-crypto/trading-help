import { invoke } from "@tauri-apps/api/core";

/**
 * Exact text of `AiEngineError::Cancelled`. The IPC contract stringifies errors,
 * so this sentence is the contract; a Rust test pins the wording so it cannot
 * drift away from this constant unnoticed.
 */
const CANCELLED_MESSAGE = "Analiza została przerwana.";

/**
 * A cancellation is a user decision, not a failure, and must never render as an
 * error panel - the same distinction as "no news source" vs "no news".
 */
export function isCancellation(err: unknown): boolean {
  return String(err).trim() === CANCELLED_MESSAGE;
}

export function newOperationId(): string {
  return crypto.randomUUID();
}

export async function cancelOperation(operationId: string): Promise<void> {
  try {
    // `false` means there was nothing left to cancel, which needs no reaction.
    await invoke<boolean>("cancel_operation", { operationId });
  } catch (err) {
    console.error("Failed to cancel the operation:", err);
  }
}
