// src-tauri/src/keychain.rs
use keyring::Entry;

const SERVICE_NAME: &str = "trading-help";
const GEMINI_KEY_USERNAME: &str = "gemini_api_key";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, GEMINI_KEY_USERNAME)
        .map_err(|e| format!("Nie udało się otworzyć magazynu kluczy systemu: {}", e))
}

/// Stores the Gemini API key in the OS keychain (Windows Credential Manager).
pub fn save_gemini_api_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Klucz API nie może być pusty.".to_string());
    }
    entry()?
        .set_password(key.trim())
        .map_err(|e| format!("Nie udało się zapisać klucza API: {}", e))
}

/// Reads the stored Gemini API key. Errors when no key is present.
pub fn get_gemini_api_key() -> Result<String, String> {
    entry()?
        .get_password()
        .map_err(|e| format!("Nie udało się odczytać klucza API: {}", e))
}

/// Whether a key is already stored; decides if onboarding is shown.
pub fn has_gemini_api_key() -> bool {
    get_gemini_api_key().is_ok()
}

/// Removes the stored key, e.g. from the reset button in settings.
pub fn delete_gemini_api_key() -> Result<(), String> {
    entry()?
        .delete_credential()
        .map_err(|e| format!("Nie udało się usunąć klucza API: {}", e))
}
