import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Panel } from "./Panel";
import { formatErrorMessage } from "../utils/format";

export function SettingsView({ onKeyDeleted }: { onKeyDeleted: () => void }) {
  // A native confirm() looked foreign in the terminal UI; this is an inline
  // confirmation in the same styling.
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logError, setLogError] = useState<string | null>(null);

  const handleOpenLogs = async () => {
    setLogError(null);
    try {
      await invoke("open_log_directory");
    } catch (err) {
      console.error("Failed to open the log directory:", err);
      setLogError(formatErrorMessage(err));
    }
  };

  const handleDelete = async () => {
    setError(null);
    try {
      await invoke("delete_gemini_api_key");
      onKeyDeleted();
    } catch (err) {
      console.error("Failed to delete the key:", err);
      setError(formatErrorMessage(err));
      setConfirming(false);
    }
  };

  return (
    <div className="space-y-3">
      <Panel title="Klucz API Gemini">
        <p className="text-xs text-term-dim mb-3">
          Klucz jest zapisany w natywnym magazynie kluczy systemu i nigdy nie opuszcza tego komputera.
        </p>

        {confirming ? (
          <div className="border border-term-red/50 bg-term-red/10 p-3 space-y-3">
            <p className="text-xs text-term-text">
              Usunąć zapisany klucz API? Aplikacja wróci do ekranu powitalnego i poprosi o nowy klucz.
            </p>
            <div className="flex gap-2">
              <button
                onClick={handleDelete}
                className="px-3 py-1.5 border border-term-red text-term-red text-xs font-bold uppercase tracking-wide hover:bg-term-red/10 transition-colors"
              >
                Usuń klucz
              </button>
              <button
                onClick={() => setConfirming(false)}
                className="px-3 py-1.5 border border-term-line-strong text-term-dim text-xs font-bold uppercase tracking-wide hover:text-term-text transition-colors"
              >
                Anuluj
              </button>
            </div>
          </div>
        ) : (
          <button
            onClick={() => setConfirming(true)}
            className="px-4 py-2 border border-term-line-strong text-term-dim text-xs font-bold uppercase tracking-wide hover:border-term-red hover:text-term-red transition-colors"
          >
            Zmień klucz API
          </button>
        )}

        {error && <p className="text-xs text-term-red font-mono mt-3">{error}</p>}
      </Panel>

      <Panel title="Diagnostyka">
        <p className="text-xs text-term-dim mb-3">
          Aplikacja zapisuje lokalny plik z logiem błędów — bez kluczy API i bez treści zapytań do AI. Nic nie jest
          nigdzie wysyłane; plik możesz dołączyć do zgłoszenia problemu.
        </p>
        <button
          onClick={handleOpenLogs}
          className="px-4 py-2 border border-term-line-strong text-term-dim text-xs font-bold uppercase tracking-wide hover:border-term-cyan hover:text-term-cyan transition-colors"
        >
          Otwórz folder z logami
        </button>
        {logError && <p className="text-xs text-term-red font-mono mt-3">{logError}</p>}
      </Panel>
    </div>
  );
}
