//! Typ błędu wspólny dla całej warstwy komend Tauri. Każdy submoduł
//! (cross_market, precious_metals, briefing, tactics) zwraca
//! `Result<T, CommandError>`. Stringifikacja na `String` (kontrakt IPC z
//! frontendem) dzieje się WYŁĄCZNIE w cienkich wrapperach `#[tauri::command]`
//! w `mod.rs`, nigdy tutaj ani w submodułach logiki.

use thiserror::Error;

/// błędy warstwy komend - #[tauri::command] i tak zwraca String do frontendu,
/// stringify dzieje się w jednym miejscu na końcu
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Błąd pobierania danych rynkowych: {0}")]
    MarketData(String),

    #[error(transparent)]
    Ai(#[from] crate::ai_engine::AiEngineError),

    #[error("Błąd zapisu/odczytu lokalnej historii: {0}")]
    Storage(String),

    #[error("Błąd magazynu kluczy systemu: {0}")]
    Keychain(String),

    #[error("Brak danych do analizy indeksów")]
    NoStrongestPair,

    #[error("Nieznany instrument: {0}")]
    UnknownInstrument(String),
}
