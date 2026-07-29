//! Cooperative cancellation for long AI calls.
//!
//! Deliberately hand-written rather than `tokio_util::sync::CancellationToken`:
//! the registry keyed by operation id has to exist either way, and the ready-made
//! crate solves only the part that was not missing. A new node in the audit tree
//! for ~30 lines is a bad trade.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Notify;

/// Ids come from the frontend, so they are treated as untrusted input: a bounded
/// shape keeps the registry from being filled with arbitrary strings.
const MAX_ID_LEN: usize = 64;
const MIN_ID_LEN: usize = 8;

/// One operation per instrument in flight is the realistic worst case. Well
/// above that means entries are not being released, and the cap keeps the map
/// bounded regardless of the cause.
const MAX_ACTIVE_OPERATIONS: usize = 32;

#[derive(Default)]
struct Inner {
    flag: AtomicBool,
    notify: Notify,
}

/// Cheap to clone; every clone observes the same cancellation.
#[derive(Clone, Default)]
pub struct CancelToken {
    inner: Arc<Inner>,
}

impl CancelToken {
    pub fn cancel(&self) {
        self.inner.flag.store(true, Ordering::Release);
        self.inner.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.flag.load(Ordering::Acquire)
    }

    /// Resolves as soon as the operation is cancelled, including when it already
    /// was before this was awaited.
    pub async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            // Interest must be registered before the final check: `notify_waiters`
            // only wakes existing waiters, so checking afterwards would miss a
            // cancellation landing in between.
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

fn registry() -> &'static Mutex<HashMap<String, CancelToken>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, CancelToken>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_valid_operation_id(id: &str) -> bool {
    (MIN_ID_LEN..=MAX_ID_LEN).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Removes the entry when the operation ends - on success, on error, and while
/// unwinding from a panic, since `Drop` runs during unwind. The registry cannot
/// leak an entry as long as the guard stays on the stack of the command.
pub struct OperationGuard {
    id: Option<String>,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            if let Ok(mut guard) = registry().lock() {
                guard.remove(&id);
            }
        }
    }
}

/// Registers a cancellable operation. A malformed id or a full registry yields a
/// token that simply never fires: the operation still runs and returns its
/// result, it just cannot be interrupted. Failing the whole call because
/// cancellation is unavailable would be a worse trade.
pub fn register(operation_id: &str) -> (CancelToken, OperationGuard) {
    let token = CancelToken::default();

    if !is_valid_operation_id(operation_id) {
        log::warn!("Rejected malformed operation id (len={})", operation_id.len());
        return (token, OperationGuard { id: None });
    }

    match registry().lock() {
        Ok(mut guard) => {
            if guard.len() >= MAX_ACTIVE_OPERATIONS && !guard.contains_key(operation_id) {
                log::warn!("Operation registry full ({} entries); running uncancellable", guard.len());
                return (token, OperationGuard { id: None });
            }
            guard.insert(operation_id.to_string(), token.clone());
            (token, OperationGuard { id: Some(operation_id.to_string()) })
        }
        Err(_) => (token, OperationGuard { id: None }),
    }
}

/// Returns whether anything was cancelled. An unknown id is an ordinary outcome,
/// not an error: the window may have been reopened, or the call may have already
/// finished between the click and this arriving.
pub fn cancel(operation_id: &str) -> bool {
    if !is_valid_operation_id(operation_id) {
        return false;
    }
    let Ok(guard) = registry().lock() else { return false };
    match guard.get(operation_id) {
        Some(token) => {
            token.cancel();
            true
        }
        None => false,
    }
}

#[cfg(test)]
pub(crate) fn active_operations() -> usize {
    registry().lock().map(|g| g.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn cancelling_an_unknown_id_is_false_not_a_panic() {
        assert!(!cancel("00000000-0000-0000-0000-000000000000"));
        assert!(!cancel(""));
        assert!(!cancel("!@#$%^&*()"));
    }

    #[test]
    fn malformed_ids_are_never_stored() {
        let before = active_operations();
        let (_token, guard) = register("short");
        assert_eq!(active_operations(), before, "zbyt krótkie id nie może wejść do rejestru");
        drop(guard);

        let (_token, guard) = register(&"x".repeat(MAX_ID_LEN + 1));
        assert_eq!(active_operations(), before, "zbyt długie id nie może wejść do rejestru");
        drop(guard);
    }

    #[test]
    fn guard_releases_the_entry_even_on_an_error_path() {
        let id = "guarded-error-path-1234";
        let before = active_operations();

        let result: Result<(), &str> = (|| {
            let (_token, _guard) = register(id);
            assert_eq!(active_operations(), before + 1);
            Err("coś poszło nie tak")
        })();

        assert!(result.is_err());
        assert_eq!(active_operations(), before, "wpis został w rejestrze po błędzie");
        assert!(!cancel(id));
    }

    #[tokio::test]
    async fn cancelled_resolves_immediately_when_already_cancelled() {
        let token = CancelToken::default();
        token.cancel();

        let started = std::time::Instant::now();
        tokio::time::timeout(Duration::from_millis(200), token.cancelled())
            .await
            .expect("cancelled() powinno wrócić natychmiast");
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn cancel_by_id_wakes_a_waiting_operation() {
        let id = "wakes-a-waiter-abcd1234";
        let (token, _guard) = register(id);

        let waiter = tokio::spawn(async move { token.cancelled().await });
        // Yield so the waiter reaches the await before the cancellation lands.
        tokio::task::yield_now().await;
        assert!(cancel(id));

        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("oczekujący powinien zostać obudzony")
            .unwrap();
    }
}
