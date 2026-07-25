// src-tauri/src/tactic_store.rs
use crate::models::TrackedTactic;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn tactics_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("tactics.json"))
}

pub fn load_all(app: &AppHandle) -> Vec<TrackedTactic> {
    let Ok(path) = tactics_path(app) else { return Vec::new(); };
    let Ok(content) = fs::read_to_string(path) else { return Vec::new(); };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_all(app: &AppHandle, tactics: &[TrackedTactic]) -> Result<(), String> {
    let path = tactics_path(app)?;
    let json = serde_json::to_string_pretty(tactics).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn append(app: &AppHandle, tactic: TrackedTactic) -> Result<(), String> {
    let mut all = load_all(app);
    all.push(tactic);
    save_all(app, &all)
}
