// src-tauri/src/history_store.rs
use crate::models::Snapshot;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn snapshot_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("last_snapshot.json"))
}

pub fn load_last_snapshot(app: &AppHandle) -> Option<Snapshot> {
    let path = snapshot_path(app).ok()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_snapshot(app: &AppHandle, snapshot: &Snapshot) -> Result<(), String> {
    let path = snapshot_path(app)?;
    let json = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}