// src-tauri/src/tactic_store.rs
//
// tactics.json holds the only data in this app that cannot be recreated: a
// snapshot can be recomputed and a briefing regenerated, but a track record is
// a measurement of the past. Every write therefore goes through a temporary
// file in the same directory followed by a rename, and every read-modify-write
// cycle runs under one lock.
use crate::models::TrackedTactic;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::{AppHandle, Manager};

/// Guards the whole read-modify-write cycle, not just the write: two commands
/// generating tactics for different instruments would otherwise both read the
/// old file and the later write would drop the earlier entry. One process owns
/// one store, so a single lock is enough. Nothing awaits while it is held -
/// every function here is synchronous.
fn file_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = LOCK.get_or_init(|| Mutex::new(()));
    // A panic elsewhere must not permanently disable persistence.
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn tactics_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("tactics.json"))
}

/// Assumes the caller holds the lock.
fn read_unlocked(path: &Path) -> Vec<TrackedTactic> {
    let Ok(content) = fs::read_to_string(path) else { return Vec::new(); };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Assumes the caller holds the lock. Returns whether anything was written.
///
/// `before_rename` exists so the crash window between "temp file written" and
/// "rename done" can be exercised by tests; production passes a no-op.
fn write_unlocked(
    path: &Path,
    tactics: &[TrackedTactic],
    before_rename: impl FnOnce() -> Result<(), String>,
) -> Result<bool, String> {
    let json = serde_json::to_string_pretty(tactics)
        .map_err(|e| format!("Nie udało się przygotować danych taktyk do zapisu: {e}"))?;

    // Skipping an identical write is not an optimisation: the track record is
    // recomputed on every stats view, so without this the most frequent path
    // would also be the one most often rewriting the file it can corrupt.
    if fs::read_to_string(path).is_ok_and(|existing| existing == json) {
        return Ok(false);
    }

    // The temporary file must sit in the destination directory: a rename within
    // one volume is atomic, while moving across volumes (a system temp dir) is
    // a copy and loses that guarantee.
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, &json)
        .map_err(|e| format!("Nie udało się zapisać pliku tymczasowego taktyk: {e}"))?;

    if let Err(e) = before_rename() {
        // Leaving the temp file behind would make the next write look like a
        // partial state; the previous tactics.json is untouched either way.
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    fs::rename(&temp_path, path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        format!("Nie udało się zapisać taktyk: {e}")
    })?;

    Ok(true)
}

pub fn load_from(path: &Path) -> Vec<TrackedTactic> {
    let _guard = file_lock();
    read_unlocked(path)
}

pub fn save_to(path: &Path, tactics: &[TrackedTactic]) -> Result<bool, String> {
    let _guard = file_lock();
    write_unlocked(path, tactics, || Ok(()))
}

pub fn append_to(path: &Path, tactic: TrackedTactic) -> Result<bool, String> {
    let _guard = file_lock();
    let mut all = read_unlocked(path);
    all.push(tactic);
    write_unlocked(path, &all, || Ok(()))
}

pub fn load_all(app: &AppHandle) -> Vec<TrackedTactic> {
    let Ok(path) = tactics_path(app) else { return Vec::new(); };
    load_from(&path)
}

pub fn save_all(app: &AppHandle, tactics: &[TrackedTactic]) -> Result<bool, String> {
    save_to(&tactics_path(app)?, tactics)
}

pub fn append(app: &AppHandle, tactic: TrackedTactic) -> Result<bool, String> {
    append_to(&tactics_path(app)?, tactic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir(name: &str) -> PathBuf {
        // Each test gets its own directory; the store lock is global, so shared
        // paths would make tests observe each other's writes.
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("trading_help_store_{name}_{unique}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tactic(id: &str) -> TrackedTactic {
        TrackedTactic {
            id: id.to_string(),
            instrument: "GOLD".to_string(),
            scenario: "bull".to_string(),
            reference_price: 100.0,
            entry_pct: 0.0,
            target_pct: 2.0,
            stop_loss_pct: -1.0,
            generated_at: 1000,
            verified_24h: None,
            verified_7d: None,
        }
    }

    #[test]
    fn round_trips_tactics() {
        let path = temp_dir("roundtrip").join("tactics.json");
        assert!(save_to(&path, &[tactic("a"), tactic("b")]).unwrap());

        let loaded = load_from(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "a");
    }

    #[test]
    fn missing_or_corrupt_file_reads_as_empty_rather_than_failing() {
        let dir = temp_dir("corrupt");
        let path = dir.join("tactics.json");
        assert!(load_from(&path).is_empty());

        fs::write(&path, "{ to nie jest tablica taktyk").unwrap();
        assert!(load_from(&path).is_empty());
    }

    #[test]
    fn append_keeps_existing_entries() {
        let path = temp_dir("append").join("tactics.json");
        append_to(&path, tactic("first")).unwrap();
        append_to(&path, tactic("second")).unwrap();

        let loaded = load_from(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].id, "second");
    }

    #[test]
    fn a_failure_before_rename_leaves_the_previous_file_intact() {
        let path = temp_dir("crash").join("tactics.json");
        save_to(&path, &[tactic("original")]).unwrap();
        let before = fs::read_to_string(&path).unwrap();

        let result = {
            let _guard = file_lock();
            write_unlocked(&path, &[tactic("replacement")], || {
                Err("przerwane w połowie zapisu".to_string())
            })
        };

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before, "plik został uszkodzony");
        assert_eq!(load_from(&path)[0].id, "original");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "plik tymczasowy powinien zostać posprzątany"
        );
    }

    #[test]
    fn identical_content_is_not_written_again() {
        let path = temp_dir("nochange").join("tactics.json");
        let tactics = [tactic("a")];

        assert!(save_to(&path, &tactics).unwrap(), "pierwszy zapis powinien dojść do skutku");
        assert!(!save_to(&path, &tactics).unwrap(), "identyczna treść nie powinna być zapisywana ponownie");

        let modified = [tactic("a"), tactic("b")];
        assert!(save_to(&path, &modified).unwrap());
    }

    #[test]
    fn concurrent_appends_do_not_lose_entries() {
        let path = temp_dir("concurrent").join("tactics.json");
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    append_to(&path, tactic(&format!("t{i}"))).unwrap();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }

        let loaded = load_from(&path);
        assert_eq!(loaded.len(), 8, "równoległe zapisy zgubiły wpisy");
        let mut ids: Vec<_> = loaded.iter().map(|t| t.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8);
    }
}
