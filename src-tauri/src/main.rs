// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    trading_help::run(); // Wywołujemy funkcję run z lib.rs
}