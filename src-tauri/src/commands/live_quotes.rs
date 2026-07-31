//! Live quotes for the watchlist: one light chart-metadata request per
//! instrument, no candles, no cache, no AI, no rate limit. Feeds the ticker
//! tape on a short interval, independently of the heavy analytical context.
//! No #[tauri::command] here.

use crate::{market_engine, models};
use super::error::CommandError;
use super::instruments::yahoo_symbol_for;
use tokio::task::JoinError;

/// What one joined task yields: the outer `Result` comes from tokio (did the
/// task finish at all), the inner one from the fetch (did the provider answer).
/// Named because it appears in a signature and in every test input below.
type JoinedQuote = Result<Result<models::LiveQuote, market_engine::MarketDataError>, JoinError>;

/// The list arrives from the frontend and is treated as untrusted input (the
/// same rule as operation ids): a bounded size keeps one call from fanning out
/// into an arbitrary number of network requests.
const MAX_INSTRUMENTS: usize = 32;

pub(crate) async fn get_live_quotes_inner(
    instruments: Vec<String>,
) -> Result<Vec<models::LiveQuote>, CommandError> {
    if instruments.len() > MAX_INSTRUMENTS {
        return Err(CommandError::InvalidTicker(format!(
            "zbyt wiele instrumentów w jednym zapytaniu: {}",
            instruments.len()
        )));
    }

    // Everything resolves before anything is fetched: an unknown id is an
    // error, never a skip - a silently missing quote reads as a market outage
    // rather than the caller bug it actually is.
    let resolved: Vec<(String, &'static str)> = instruments
        .into_iter()
        .map(|id| {
            yahoo_symbol_for(&id)
                .map(|symbol| (id.clone(), symbol))
                .ok_or(CommandError::UnknownInstrument(id))
        })
        .collect::<Result<_, _>>()?;

    let handles: Vec<_> = resolved
        .into_iter()
        .map(|(id, symbol)| {
            tokio::spawn(async move { market_engine::fetch_live_quote(&id, symbol).await })
        })
        .collect();

    // Awaited in order rather than through a combinator: the tasks are already
    // running concurrently from the spawn above, so this only gathers them.
    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await);
    }

    collect_live_quotes(results)
}

/// Decides what a mixed bag of successes and failures means. Separate from the
/// spawning above because that decision owns no I/O and no tokio: it is a pure
/// function over outcomes, which is also the only shape in which it can be
/// tested.
///
/// `JoinError` has no public constructor - the only way to obtain one is to
/// await a task that actually panicked - so the tests below build their input
/// from a real panicking task rather than from a mock.
fn collect_live_quotes(results: Vec<JoinedQuote>) -> Result<Vec<models::LiveQuote>, CommandError> {
    let mut quotes = Vec::new();
    let mut first_error: Option<CommandError> = None;

    for result in results {
        match result {
            Ok(Ok(quote)) => quotes.push(quote),
            Ok(Err(e)) => {
                log::warn!("Live quote failed: {e}");
                if first_error.is_none() {
                    first_error = Some(e.into());
                }
            }
            // A task that never returned is still a failure, so it feeds
            // first_error on the same terms as a fetch error. Logging alone let
            // a round where every task panicked return an empty Ok, which the
            // frontend silences - the exact "frozen tape showing stale prices"
            // symptom this release exists to remove.
            Err(join_error) => {
                log::warn!("Live quote task failed: {join_error}");
                if first_error.is_none() {
                    first_error = Some(CommandError::LiveQuoteTaskFailed);
                }
            }
        }
    }

    // Partial results pass through: the tape keeps the previous value and its
    // visible age explains the gap. Zero results with an error in hand is a
    // genuine outage and is reported as one.
    if quotes.is_empty() {
        if let Some(error) = first_error {
            return Err(error);
        }
    }
    Ok(quotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Network-dependent paths are exercised by hand; these cover everything
    // that must fail (or succeed) before a single request leaves the machine,
    // plus the whole of `collect_live_quotes`, which needs no network at all.
    //
    // EXPECTED OUTPUT: a green run of this module prints panic messages and
    // backtrace hints to stderr. They come from the tasks that `join_error()`
    // panics on purpose - the only way to obtain a real `JoinError` - and are
    // not a failure. Read the `test result:` line, not the stderr noise.

    /// Synthetic figures throughout: the aggregation never inspects a price, so
    /// realistic quotes would only suggest it does.
    fn quote(instrument: &str) -> models::LiveQuote {
        models::LiveQuote {
            instrument: instrument.to_string(),
            price: 100.0,
            previous_close: 99.0,
            daily_change_pct: 1.0,
            market_time: 1_785_400_000,
        }
    }

    fn fetch_error(symbol: &str) -> market_engine::MarketDataError {
        market_engine::MarketDataError::NoData { symbol: symbol.to_string() }
    }

    /// The only way to obtain a `JoinError`: await a task that really panicked.
    /// The panic message it prints to stderr during the run is expected.
    async fn join_error() -> JoinError {
        tokio::spawn(async { panic!("celowa panika w teście") })
            .await
            .expect_err("zadanie miało spanikować")
    }

    #[tokio::test]
    async fn every_task_panicking_is_an_error_not_an_empty_list() {
        // The regression this whole change exists for: an empty Ok here is
        // silenced by the frontend and freezes the tape on stale prices.
        let results: Vec<JoinedQuote> = vec![Err(join_error().await), Err(join_error().await)];

        let result = collect_live_quotes(results);

        assert!(
            matches!(result, Err(CommandError::LiveQuoteTaskFailed)),
            "wszystkie zadania padły, a komenda nie zgłosiła błędu"
        );
    }

    #[test]
    fn every_fetch_failing_reports_the_providers_error() {
        let results: Vec<JoinedQuote> =
            vec![Ok(Err(fetch_error("GC=F"))), Ok(Err(fetch_error("SI=F")))];

        match collect_live_quotes(results) {
            Err(CommandError::MarketData(e)) => {
                // The first failure is the one reported, so the symbol pins it.
                assert!(e.to_string().contains("GC=F"), "otrzymano {e}");
            }
            other => panic!("oczekiwano błędu dostawcy, otrzymano {other:?}"),
        }
    }

    #[tokio::test]
    async fn one_panic_among_successes_still_returns_the_successes() {
        let results: Vec<JoinedQuote> = vec![
            Ok(Ok(quote("GOLD"))),
            Err(join_error().await),
            Ok(Ok(quote("SILVER"))),
        ];

        let quotes = collect_live_quotes(results).expect("wyniki częściowe mają przechodzić");

        let instruments: Vec<&str> = quotes.iter().map(|q| q.instrument.as_str()).collect();
        assert_eq!(instruments, vec!["GOLD", "SILVER"]);
    }

    #[test]
    fn one_fetch_failure_among_successes_still_returns_the_successes() {
        let results: Vec<JoinedQuote> =
            vec![Ok(Err(fetch_error("GC=F"))), Ok(Ok(quote("NASDAQ")))];

        let quotes = collect_live_quotes(results).expect("wyniki częściowe mają przechodzić");

        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].instrument, "NASDAQ");
    }

    #[tokio::test]
    async fn the_first_failure_wins_and_later_ones_do_not_overwrite_it() {
        // Fetch failure first: a later panic must not replace it.
        let fetch_first: Vec<JoinedQuote> =
            vec![Ok(Err(fetch_error("GC=F"))), Err(join_error().await)];
        match collect_live_quotes(fetch_first) {
            Err(CommandError::MarketData(e)) => assert!(e.to_string().contains("GC=F")),
            other => panic!("pierwszy błąd miał wygrać, otrzymano {other:?}"),
        }

        // Panic first: a later fetch failure must not replace it either.
        let panic_first: Vec<JoinedQuote> =
            vec![Err(join_error().await), Ok(Err(fetch_error("SI=F")))];
        assert!(matches!(
            collect_live_quotes(panic_first),
            Err(CommandError::LiveQuoteTaskFailed)
        ));
    }

    #[test]
    fn an_empty_input_is_an_empty_success() {
        // Nothing was requested, so nothing failed - the caller decides the
        // list, and an empty watchlist is its own doing, not an outage.
        let quotes = collect_live_quotes(Vec::new()).expect("pusty wektor nie jest awarią");
        assert!(quotes.is_empty());
    }

    #[tokio::test]
    async fn unknown_instrument_is_an_error_before_any_fetch() {
        let result = get_live_quotes_inner(vec!["GOLD".to_string(), "BITCOIN".to_string()]).await;
        assert!(matches!(result, Err(CommandError::UnknownInstrument(id)) if id == "BITCOIN"));
    }

    #[tokio::test]
    async fn empty_watchlist_yields_empty_quotes_not_an_error() {
        let quotes = get_live_quotes_inner(Vec::new()).await.unwrap();
        assert!(quotes.is_empty());
    }

    #[tokio::test]
    async fn oversized_request_is_rejected() {
        let too_many: Vec<String> = (0..MAX_INSTRUMENTS + 1).map(|i| format!("X{i}")).collect();
        let result = get_live_quotes_inner(too_many).await;
        assert!(matches!(result, Err(CommandError::InvalidTicker(_))));
    }

    /// The aggregation path itself cannot be reached from here (see the note in
    /// the module tests above), so this pins the half that is reachable: the
    /// message a failed task would surface. A `JoinError` renders as a Rust
    /// panic message, and none of that may travel to the UI.
    #[test]
    fn task_failure_message_carries_no_panic_details() {
        let message = CommandError::LiveQuoteTaskFailed.to_string();

        for leak in ["joinerror", "panic", "unwrap", "src/", ".rs"] {
            assert!(
                !message.to_lowercase().contains(leak),
                "komunikat dla usera zawiera szczegół techniczny '{leak}': {message}"
            );
        }
        assert!(message.contains("logu"), "komunikat ma kierować po szczegóły do logu");
    }

    /// A task failure and a provider failure are different faults and must stay
    /// distinguishable - one points at this application, the other at Yahoo.
    #[test]
    fn task_failure_is_a_distinct_variant_from_a_fetch_failure() {
        let task_failed = CommandError::LiveQuoteTaskFailed;
        let fetch_failed = CommandError::MarketData(crate::market_engine::MarketDataError::NoData {
            symbol: "TEST".to_string(),
        });

        assert!(!matches!(task_failed, CommandError::MarketData(_)));
        assert_ne!(task_failed.to_string(), fetch_failed.to_string());
    }
}
