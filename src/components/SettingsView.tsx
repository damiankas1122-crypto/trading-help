import { invoke } from "@tauri-apps/api/core";
import { Panel } from "./Panel";

export function SettingsView({ onKeyDeleted }: { onKeyDeleted: () => void }) {
  const handleDelete = async () => {
    if (!confirm("Usunąć zapisany klucz API i wprowadzić nowy?")) return;
    try {
      await invoke("delete_gemini_api_key");
      onKeyDeleted();
    } catch (err) {
      console.error("Błąd usuwania klucza:", err);
    }
  };

  return (
    <div className="space-y-3">
      <Panel title="Klucz API Gemini">
        <p className="text-xs text-term-dim mb-3">
          Klucz jest zapisany w natywnym magazynie kluczy systemu i nigdy nie opuszcza tego komputera.
        </p>
        <button
          onClick={handleDelete}
          className="px-4 py-2 border border-term-line-strong text-term-dim text-xs font-bold uppercase tracking-wide hover:border-term-red hover:text-term-red transition-colors"
        >
          Zmień klucz API
        </button>
      </Panel>
    </div>
  );
}
