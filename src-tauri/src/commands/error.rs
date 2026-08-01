//! Error type shared by the whole Tauri command layer. Every submodule returns
//! `Result<T, CommandError>`. Conversion to `String` (the IPC contract with the
//! frontend) happens only in the thin `#[tauri::command]` wrappers in `mod.rs`,
//! never here or in the logic submodules.

use thiserror::Error;

/// Command-layer errors. Since `#[tauri::command]` returns `String` to the
/// frontend anyway, stringification is centralised at the boundary.
#[derive(Error, Debug)]
pub enum CommandError {
    /// Transparent: the market engine already distinguishes a source outage from
    /// an instrument with too little history, and a wrapping prefix would blur
    /// that back into one "data error".
    #[error(transparent)]
    MarketData(#[from] crate::market_engine::MarketDataError),

    #[error(transparent)]
    Ai(#[from] crate::ai_engine::AiEngineError),

    #[error("Błąd zapisu/odczytu lokalnej historii: {0}")]
    Storage(String),

    #[error("Błąd magazynu kluczy systemu: {0}")]
    Keychain(String),

    /// A spawned quote task never returned - it panicked or was cancelled. Kept
    /// apart from a fetch failure: the fault is inside the application, not at
    /// the provider. Carries no payload because `JoinError` renders as a Rust
    /// panic message, which belongs in the log and never in the UI.
    #[error("Pobieranie notowań nie zostało dokończone. Szczegóły zapisano w logu.")]
    LiveQuoteTaskFailed,

    #[error("Nieznany instrument: {0}")]
    UnknownInstrument(String),

    #[error("Nieprawidłowy symbol tickera: {0}")]
    InvalidTicker(String),

    /// A tactic priced at zero would score every target as hit, so it must never
    /// reach the store that feeds the track record.
    #[error("Nieprawidłowa cena odniesienia dla {instrument}: {price}. Taktyka nie została zapisana.")]
    InvalidReferencePrice { instrument: String, price: f64 },
}
