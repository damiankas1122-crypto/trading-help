import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { formatErrorMessage } from "../utils/format";

const API_KEY_URL = "https://aistudio.google.com/apikey";

export function ApiKeyOnboarding({ onSaved }: { onSaved: () => void }) {
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    if (!apiKey.trim()) {
      setError("Wklej swój klucz API Gemini przed zapisaniem.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await invoke("save_gemini_api_key", { key: apiKey.trim() });
      onSaved();
    } catch (err) {
      console.error("Failed to save the API key:", err);
      setError(formatErrorMessage(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="h-screen w-screen bg-term-bg text-term-text flex items-center justify-center p-6 font-mono">
      <div className="max-w-md w-full bg-term-panel border border-term-line-strong p-8 space-y-5">
        <div>
          <h1 className="text-term-amber text-lg font-black tracking-[0.15em] uppercase mb-2">
            Witaj w Trading Help
          </h1>
          <p className="text-term-dim text-sm leading-relaxed">
            Aby generować analizy AI, potrzebny jest Twój własny klucz API Google Gemini.
            Klucz zostanie zapisany bezpiecznie w natywnym magazynie kluczy Twojego systemu
            i nigdy nie opuści Twojego komputera.
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-xs text-term-faint uppercase tracking-wide font-bold">
            Klucz API Gemini
          </label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="AIza..."
            className="w-full bg-black border border-term-line-strong px-3 py-2 text-sm text-term-text focus:outline-none focus:border-term-amber"
          />
        </div>

        {error && (
          <p className="text-term-red text-xs whitespace-pre-wrap">{error}</p>
        )}

        <button
          onClick={handleSave}
          disabled={saving}
          className="w-full px-4 py-2.5 bg-term-amber/10 border border-term-amber text-term-amber text-xs font-bold uppercase tracking-wider hover:bg-term-amber/20 transition-colors disabled:opacity-50"
        >
          {saving ? "Zapisuję..." : "Zapisz i uruchom aplikację"}
        </button>

        <p className="text-term-faint text-[11px]">
          Nie masz jeszcze klucza? Wygeneruj go bezpłatnie na{" "}
          <button
            onClick={() => {
              // A missing browser or permission must not break onboarding.
              openUrl(API_KEY_URL).catch((err) =>
                console.warn("Failed to open the browser:", err)
              );
            }}
            className="text-term-cyan underline underline-offset-2 hover:text-term-text transition-colors"
          >
            aistudio.google.com/apikey
          </button>
        </p>
      </div>
    </div>
  );
}
