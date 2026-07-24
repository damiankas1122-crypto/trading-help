// src-tauri/src/keychain.rs
use keyring::Entry;

const SERVICE_NAME: &str = "trading-help";
const GEMINI_KEY_USERNAME: &str = "gemini_api_key";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, GEMINI_KEY_USERNAME)
        .map_err(|e| format!("Nie udało się otworzyć magazynu kluczy systemu: {}", e))
}

/// Zapisuje klucz API Gemini w natywnym magazynie kluczy systemu
/// (Windows Credential Manager na Windows).
pub fn save_gemini_api_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Klucz API nie może być pusty.".to_string());
    }
    entry()?
        .set_password(key.trim())
        .map_err(|e| format!("Nie udało się zapisać klucza API: {}", e))
}

/// Odczytuje zapisany klucz API Gemini. Zwraca błąd jeśli klucz nie istnieje.
pub fn get_gemini_api_key() -> Result<String, String> {
    entry()?
        .get_password()
        .map_err(|e| format!("Nie udało się odczytać klucza API: {}", e))
}

/// Sprawdza czy klucz API jest już zapisany (do decyzji: pokazać onboarding czy nie).
pub fn has_gemini_api_key() -> bool {
    get_gemini_api_key().is_ok()
}

/// Usuwa zapisany klucz API (np. przycisk "zresetuj klucz" w ustawieniach).
pub fn delete_gemini_api_key() -> Result<(), String> {
    entry()?
        .delete_credential()
        .map_err(|e| format!("Nie udało się usunąć klucza API: {}", e))
}
