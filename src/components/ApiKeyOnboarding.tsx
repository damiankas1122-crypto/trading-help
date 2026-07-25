import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
      console.error("Błąd zapisu klucza API:", err);
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="h-screen w-screen bg-[#05050a] text-white flex items-center justify-center p-6">
      <div className="max-w-md w-full bg-[#0a0a1a]/80 rounded-2xl border border-cyan-900/40 p-8 space-y-5">
        <div>
          <h1 className="text-cyan-400 text-lg font-black tracking-[0.15em] uppercase mb-2">
            Witaj w Trading Help
          </h1>
          <p className="text-slate-400 text-sm font-mono leading-relaxed">
            Aby generować analizy AI, potrzebny jest Twój własny klucz API Google Gemini.
            Klucz zostanie zapisany bezpiecznie w natywnym magazynie kluczy Twojego systemu
            i nigdy nie opuści Twojego komputera.
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-xs text-slate-500 uppercase tracking-wide font-bold">
            Klucz API Gemini
          </label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="AIza..."
            className="w-full bg-black/40 border border-cyan-900/50 rounded-lg px-3 py-2 text-sm font-mono text-slate-200 focus:outline-none focus:border-cyan-500"
          />
        </div>

        {error && (
          <p className="text-red-400 text-xs font-mono whitespace-pre-wrap">{error}</p>
        )}

        <button
          onClick={handleSave}
          disabled={saving}
          className="w-full px-4 py-2.5 rounded bg-cyan-500/20 border border-cyan-500 text-cyan-300 text-xs font-bold uppercase tracking-wider hover:bg-cyan-500/30 transition-colors disabled:opacity-50"
        >
          {saving ? "Zapisuję..." : "Zapisz i uruchom aplikację"}
        </button>

        <p className="text-slate-600 text-[11px] font-mono">
          Nie masz jeszcze klucza? Wygeneruj go bezpłatnie na{" "}
          <span className="text-cyan-500">aistudio.google.com/apikey</span>
        </p>
      </div>
    </div>
  );
}
