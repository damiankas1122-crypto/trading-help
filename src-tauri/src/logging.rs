//! Local, rolling, size-limited log file. Nothing leaves the machine: no
//! telemetry, no network, no background upload. The user attaches the file to a
//! bug report by hand, which is exactly why every record passes through
//! `scrub()` before it is written - the file must be safe to send by
//! construction, not because whoever wrote a given call site was careful.
//!
//! Owning the sink is what buys that property: `tracing-subscriber`, `fern` and
//! `tauri-plugin-log` can all drop a record, but none of them can rewrite its
//! contents. Size-based rotation and unit-testable rotation come from the same
//! decision.

use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use time::OffsetDateTime;

const LOG_FILE_NAME: &str = "trading-help.log";

/// 1 MB per file, two rotated copies (3 MB total). At Info level everyday use
/// writes a few dozen lines a day (~5 KB), so one file holds months of history,
/// while three files survive a day of error looping - the day the user actually
/// reports something. The whole set stays small enough to email.
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const ROTATED_COPIES: usize = 2;

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

/// Secrets recognisable on their own, without a surrounding key name. Add a new
/// provider's key prefix here when its shape is distinctive.
const SECRET_PREFIXES: &[&str] = &[
    "AIza", // Google / Gemini API key
];

/// Markers after which the value is a secret, whatever it looks like. Finnhub
/// passes its token as a query parameter, so any logged URL would otherwise leak
/// it - that path will exist before anyone thinks about the log. Add new markers
/// here; matching takes the longest one at a given position, so "key=" cannot
/// shadow "api_key=".
const SECRET_VALUE_MARKERS: &[&str] = &[
    "token=",
    "key=",
    "api_key=",
    "apikey=",
    "access_token=",
    "x-goog-api-key=",
    "x-goog-api-key:",
    "authorization:",
    "bearer ",
];

const REDACTED: &str = "[REDACTED]";

fn is_secret_char(c: char) -> bool {
    // Covers base64url and hex tokens; stops at &, quotes, whitespace, braces.
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '+' | '/' | '=')
}

/// Rewrites anything that looks like credential material. Runs on every record,
/// so a future call site that logs a key still cannot leak it.
pub fn scrub(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    'outer: while !rest.is_empty() {
        // A marker only counts at a word boundary, so "monkey=" is not "key=".
        let at_boundary = out
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');

        if at_boundary {
            if let Some(marker) = longest_marker_at(rest) {
                out.push_str(&rest[..marker.len()]);
                let value_len = rest[marker.len()..]
                    .char_indices()
                    .find(|(_, c)| !is_secret_char(*c))
                    .map_or(rest.len() - marker.len(), |(i, _)| i);
                if value_len > 0 {
                    out.push_str(REDACTED);
                }
                rest = &rest[marker.len() + value_len..];
                continue 'outer;
            }

            for prefix in SECRET_PREFIXES {
                if starts_with_ascii_ignore_case(rest, prefix) {
                    let len = rest
                        .char_indices()
                        .find(|(_, c)| !is_secret_char(*c))
                        .map_or(rest.len(), |(i, _)| i);
                    // A bare prefix is a normal word; only a long run is key material.
                    if len >= prefix.len() + 20 {
                        out.push_str(REDACTED);
                        rest = &rest[len..];
                        continue 'outer;
                    }
                }
            }
        }

        // Advance by a whole character: byte stepping panics on multi-byte text.
        let step = rest.chars().next().map_or(1, char::len_utf8);
        out.push_str(&rest[..step]);
        rest = &rest[step..];
    }

    out
}

/// Compares raw bytes rather than slicing the string: markers are ASCII, but
/// `rest` may start mid-way through multi-byte text, and `&rest[..n]` panics on
/// a boundary that is not a character boundary.
fn starts_with_ascii_ignore_case(rest: &str, marker: &str) -> bool {
    rest.len() >= marker.len() && rest.as_bytes()[..marker.len()].eq_ignore_ascii_case(marker.as_bytes())
}

fn longest_marker_at(rest: &str) -> Option<&'static str> {
    SECRET_VALUE_MARKERS
        .iter()
        .filter(|marker| starts_with_ascii_ignore_case(rest, marker))
        .max_by_key(|marker| marker.len())
        .copied()
}

// ---------------------------------------------------------------------------
// Rolling file
// ---------------------------------------------------------------------------

struct RollingFile {
    path: PathBuf,
    max_bytes: u64,
    rotated_copies: usize,
    file: File,
    written: u64,
}

impl RollingFile {
    fn open(path: PathBuf, max_bytes: u64, rotated_copies: usize) -> std::io::Result<Self> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let written = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Self { path, max_bytes, rotated_copies, file, written })
    }

    fn rotated_path(&self, index: usize) -> PathBuf {
        self.path.with_extension(format!("log.{index}"))
    }

    /// Shifts .1 -> .2 and current -> .1, dropping whatever fell off the end.
    fn rotate(&mut self) -> std::io::Result<()> {
        for index in (1..=self.rotated_copies).rev() {
            let from = if index == 1 { self.path.clone() } else { self.rotated_path(index - 1) };
            let to = self.rotated_path(index);
            if from.exists() {
                let _ = fs::rename(&from, &to);
            }
        }
        self.file = OpenOptions::new().create(true).write(true).truncate(true).open(&self.path)?;
        self.written = 0;
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        let bytes = line.len() as u64 + 1;
        // Rotating before the write keeps a single record from being split
        // across two files.
        if self.written + bytes > self.max_bytes && self.written > 0 {
            self.rotate()?;
        }
        writeln!(self.file, "{line}")?;
        self.written += bytes;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

struct FileLogger {
    level: LevelFilter,
    /// `None` when the log directory could not be opened: the app keeps running
    /// without a log rather than failing to start.
    file: Mutex<Option<RollingFile>>,
}

fn timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let line = scrub(&format!(
            "{} {:<5} {} - {}",
            timestamp(),
            record.level(),
            record.target(),
            record.args()
        ));

        // Local runs read the console, not the file.
        #[cfg(debug_assertions)]
        eprintln!("{line}");

        if let Ok(mut guard) = self.file.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = file.write_line(&line);
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = file.file.flush();
            }
        }
    }
}

/// Installs the logger. Returns the log file path, or `None` when the directory
/// is unusable - in which case logging degrades to debug-build stderr only and
/// the application starts normally.
pub fn init(log_dir: &Path) -> Option<PathBuf> {
    let path = log_dir.join(LOG_FILE_NAME);
    let file = RollingFile::open(path.clone(), MAX_FILE_BYTES, ROTATED_COPIES).ok();
    let opened = file.is_some();

    let level = if cfg!(debug_assertions) { LevelFilter::Debug } else { LevelFilter::Info };
    let logger = FileLogger { level, file: Mutex::new(file) };

    // A second init (tests, a restarted setup hook) must not panic; the first
    // logger simply stays in place.
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(level);
    }

    opened.then_some(path)
}

/// First line of every run: what to compare against when a user reports a
/// problem. Key presence is a bool - never the value.
/// The model name is part of this line because a provider-side change to one
/// model looks exactly like an application bug: the 2026-07-29 outage was one
/// model answering 503 deterministically while its neighbour answered 200.
pub fn log_startup(version: &str, model: &str, stored_tactics: usize, has_gemini_key: bool) {
    log::info!(
        target: "startup",
        "trading_help {version} started; model={model}; stored_tactics={stored_tactics}; gemini_key_present={has_gemini_key}"
    );
}

/// Describes an unparsable payload by shape, never by content: model output is
/// built from prompts containing third-party RSS text, and this file gets
/// emailed. Line and column come from the serde error itself, which is what
/// actually localises the fault.
pub fn describe_payload(payload: &str) -> String {
    let trimmed = payload.trim_start();
    let prefix = if trimmed.starts_with("```") {
        "code_fence"
    } else if trimmed.starts_with('{') {
        "brace"
    } else if trimmed.starts_with('[') {
        "bracket"
    } else if trimmed.is_empty() {
        "empty"
    } else {
        "prose"
    };
    format!(
        "len={} prefix={prefix} braces={}/{}",
        payload.len(),
        payload.matches('{').count(),
        payload.matches('}').count()
    )
}

/// Debug-only excerpt. Compiled out of release builds entirely rather than
/// gated by a log level: a level can be raised by configuration later, and the
/// content would then reach a file the user forwards without reading.
#[cfg(debug_assertions)]
pub fn debug_excerpt(label: &str, payload: &str) {
    let excerpt: String = payload.chars().take(200).collect();
    eprintln!("{label} (debug excerpt, max 200 chars): {}", scrub(&excerpt));
}

#[cfg(not(debug_assertions))]
pub fn debug_excerpt(_label: &str, _payload: &str) {}

#[cfg(test)]
mod scrub_tests {
    use super::*;

    #[test]
    fn google_api_key_is_removed() {
        let line = "Gemini call failed with key AIzaSyD-1234567890abcdefghijklmnopqrstuv in header";
        let scrubbed = scrub(line);
        assert!(!scrubbed.contains("AIzaSyD-1234567890abcdefghijklmnopqrstuv"));
        assert!(scrubbed.contains(REDACTED));
    }

    #[test]
    fn query_parameter_tokens_are_removed() {
        // The Finnhub shape, which will exist before anyone revisits this file.
        let line = "GET https://finnhub.io/api/v1/company-news?symbol=AAPL&token=d9kgoohr01qkkbr95dh0";
        let scrubbed = scrub(line);
        assert!(!scrubbed.contains("d9kgoohr01qkkbr95dh0"));
        assert!(scrubbed.contains("token=[REDACTED]"));
        assert!(scrubbed.contains("symbol=AAPL"), "nie-sekretne parametry mają zostać");
    }

    #[test]
    fn longest_marker_wins_so_key_does_not_shadow_api_key() {
        let scrubbed = scrub("?api_key=abcdef123456");
        assert_eq!(scrubbed, "?api_key=[REDACTED]");
    }

    #[test]
    fn header_and_bearer_forms_are_removed() {
        assert!(!scrub("x-goog-api-key: AIzaSyD-1234567890abcdefghijklmnopqrstuv").contains("AIzaSyD"));
        assert!(!scrub("authorization: Bearer sk-live-abcdef1234567890").contains("sk-live"));
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        let line = "Skipping verification of unknown instrument: DOWJONES";
        assert_eq!(scrub(line), line);
        // A word merely ending in "key" is not a marker.
        assert_eq!(scrub("monkey=banana"), "monkey=banana");
        // A short "AIza"-like word is not key material.
        assert_eq!(scrub("AIzaShort"), "AIzaShort");
    }

    #[test]
    fn multibyte_text_does_not_panic_or_corrupt() {
        let line = "Nie udało się pobrać danych — źródło padło 🚀";
        assert_eq!(scrub(line), line);
    }
}

#[cfg(test)]
mod rolling_tests {
    use super::*;
    use log::Level;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir(name: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("trading_help_log_{name}_{unique}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rotates_once_the_size_limit_is_exceeded() {
        let dir = temp_dir("rotate");
        let path = dir.join("trading-help.log");
        let mut file = RollingFile::open(path.clone(), 64, 2).unwrap();

        for i in 0..20 {
            file.write_line(&format!("line {i} padded to make the file grow")).unwrap();
        }

        assert!(path.exists());
        assert!(dir.join("trading-help.log.1").exists(), "brak pierwszego pliku historycznego");
        assert!(dir.join("trading-help.log.2").exists(), "brak drugiego pliku historycznego");
        assert!(
            !dir.join("trading-help.log.3").exists(),
            "liczba plików historycznych powinna być ograniczona"
        );
        assert!(fs::metadata(&path).unwrap().len() <= 64);
    }

    #[test]
    fn existing_content_is_appended_to_not_truncated() {
        let dir = temp_dir("append");
        let path = dir.join("trading-help.log");
        RollingFile::open(path.clone(), 1024, 2).unwrap().write_line("first run").unwrap();
        RollingFile::open(path.clone(), 1024, 2).unwrap().write_line("second run").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("first run") && content.contains("second run"));
    }

    #[test]
    fn an_unusable_directory_degrades_to_no_file_instead_of_failing() {
        // A file where the directory should be: create_dir_all cannot succeed.
        let dir = temp_dir("blocked");
        let blocker = dir.join("not_a_dir");
        fs::write(&blocker, "blocked").unwrap();

        assert!(RollingFile::open(blocker.join("trading-help.log"), 1024, 2).is_err());

        // The logger itself keeps working with no file behind it.
        let logger = FileLogger { level: LevelFilter::Info, file: Mutex::new(None) };
        logger.log(
            &Record::builder()
                .args(format_args!("nic nie powinno wybuchnąć"))
                .level(Level::Info)
                .target("test")
                .build(),
        );
        logger.flush();
    }
}
