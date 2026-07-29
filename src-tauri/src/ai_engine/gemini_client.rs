//! Raw HTTP client for the Gemini API: request/response shape, retry with
//! backoff, RESOURCE_EXHAUSTED detection. `GeminiProvider` is currently the
//! only `AiProvider` implementation (see `mod.rs`); no other submodule should
//! call `call_gemini` directly rather than through the trait.

use super::{AiEngineError, AiProvider};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use time::OffsetDateTime;

/// Switched from gemini-3.5-flash on 2026-07-29: that model answered 503
/// UNAVAILABLE deterministically (19/19 attempts, identical body length) while
/// this one returned 200 with the same key in the same minute.
pub const GEMINI_MODEL: &str = "gemini-3.6-flash";

/// Free-tier requests per day, per project, per model, as reported by Google in
/// the quota violation. Used only for the message shown to the user.
const FREE_TIER_DAILY_LIMIT: u32 = 20;

/// One extra attempt, not two. Every attempt - including a failed one - is
/// charged against a 20/day budget, so three attempts per click left the user
/// with ~6 analyses a day instead of 20.
const MAX_SERVER_ERROR_ATTEMPTS: u32 = 2;

/// One request. Measured on 2026-07-29 with gemini-3.6-flash: three successful
/// briefing calls took 9.0 s, 12.2 s and 15.5 s (n=3), so 45 s is roughly triple
/// the slowest real answer and anything beyond it is a dead connection rather
/// than a slow model. The previous 90 s was three attempts deep before anyone
/// found out.
///
/// These timings belong to the model, not to the application: after a model
/// change they have to be measured again rather than assumed to carry over.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

/// Ceiling for the whole call: every attempt and every backoff together. Fits
/// one full attempt (up to 45 s), a 2 s backoff and a second attempt with time
/// left to finish, against the measured 9-15.5 s range above. It caps the wait
/// at ~1.25 min instead of the ~5 min that three 90 s attempts plus Google's
/// 30-60 s retryDelay could reach.
const TOTAL_BUDGET: Duration = Duration::from_secs(75);

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct QuotaViolation {
    #[serde(rename = "quotaId")]
    quota_id: Option<String>,
}

#[derive(Deserialize)]
struct GeminiErrorDetail {
    #[serde(rename = "retryDelay")]
    retry_delay: Option<String>,
    /// Present on QuotaFailure details; names which quota was hit.
    violations: Option<Vec<QuotaViolation>>,
}

#[derive(Deserialize)]
struct GeminiErrorBody {
    code: Option<u16>,
    status: Option<String>,
    details: Option<Vec<GeminiErrorDetail>>,
}

/// Which quota a 429 refers to. Google reports it in `quotaId`, e.g.
/// `GenerateRequestsPerDayPerProjectPerModel-FreeTier`. The distinction decides
/// both the message and whether retrying is worth a quota unit at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuotaScope {
    PerDay,
    PerMinute,
    Unknown,
}

fn quota_scope(quota_id: Option<&str>) -> QuotaScope {
    match quota_id {
        Some(id) if id.contains("PerDay") => QuotaScope::PerDay,
        Some(id) if id.contains("PerMinute") => QuotaScope::PerMinute,
        _ => QuotaScope::Unknown,
    }
}

/// Whether another attempt is worth a quota unit.
///
/// `repeated` means the previous attempt returned the same status and the same
/// body length. A byte-identical error is a deterministic answer, not a blip, so
/// the loop stops regardless of the status - a cheap heuristic that guards
/// against every future deterministic failure, not just the one that prompted it.
fn should_retry(code: u16, attempt: u32, quota: QuotaScope, repeated: bool) -> bool {
    if repeated {
        return false;
    }
    match code {
        // Retrying an exhausted daily quota spends a unit on a certain failure.
        429 => quota != QuotaScope::PerDay,
        503 => attempt + 1 < MAX_SERVER_ERROR_ATTEMPTS,
        _ => false,
    }
}

#[derive(Deserialize)]
struct GeminiErrorWrapper {
    error: GeminiErrorBody,
}

/// What reqwest reports about a failed request, flattened into plain data.
/// `reqwest::Error` has no public constructor, so the decision below could not
/// be unit-tested against real errors; separating facts from policy makes the
/// policy testable.
#[derive(Debug, Clone, Default)]
struct TransportFacts {
    is_timeout: bool,
    is_connect: bool,
    is_request: bool,
    is_body: bool,
    is_builder: bool,
    is_redirect: bool,
    is_decode: bool,
    /// Display text of the whole `source()` chain, lowercased.
    chain: String,
}

fn transport_facts(error: &reqwest::Error) -> TransportFacts {
    let mut chain = error.to_string();
    let mut source: Option<&dyn std::error::Error> = std::error::Error::source(error);
    while let Some(inner) = source {
        chain.push_str("; ");
        chain.push_str(&inner.to_string());
        source = inner.source();
    }

    TransportFacts {
        is_timeout: error.is_timeout(),
        is_connect: error.is_connect(),
        is_request: error.is_request(),
        is_body: error.is_body(),
        is_builder: error.is_builder(),
        is_redirect: error.is_redirect(),
        is_decode: error.is_decode(),
        chain: chain.to_lowercase(),
    }
}

/// reqwest exposes no `is_tls()`, and certificate failures arrive as
/// `is_connect()` with the real cause buried in the source chain. Matching text
/// is fragile, so it is used only to *downgrade* a decision that would otherwise
/// retry: a mis-detected marker costs one avoided retry, never a wrong retry.
const NON_RETRYABLE_CHAIN_MARKERS: &[&str] = &[
    "certificate",
    "handshake",
    "invalid peer",
    "unknown issuer",
    "self-signed",
    "untrusted",
];

/// Retrying is for faults that a second attempt can plausibly survive: a dropped
/// connection, a stalled read, a resolver that was not ready yet. It is not for
/// faults that are certain to repeat, where three attempts only mean waiting for
/// the same answer three times.
///
/// | fact | decision | why |
/// |---|---|---|
/// | `is_timeout` | retry | the classic blip |
/// | `is_connect` | retry | includes DNS; the host is a compile-time constant, so a permanent failure means the machine is offline and the next attempt may catch the link coming up |
/// | `is_request` / `is_body` | retry | connection died mid-exchange |
/// | `is_builder` | no | malformed request: a bug, identical every time |
/// | `is_redirect` | no | redirect policy, not transport |
/// | `is_decode` | no | the bytes arrived and are wrong; sending again changes nothing |
/// | TLS markers in chain | no | a bad certificate is configuration or interception, never a blip |
/// | anything unrecognised | no | unknown faults are assumed permanent, so an unclassified error fails fast |
fn is_retryable_transport(facts: &TransportFacts) -> bool {
    if facts.is_builder || facts.is_redirect || facts.is_decode {
        return false;
    }
    if NON_RETRYABLE_CHAIN_MARKERS.iter().any(|marker| facts.chain.contains(marker)) {
        return false;
    }
    facts.is_timeout || facts.is_connect || facts.is_request || facts.is_body
}

// ---------------------------------------------------------------------------
// Daily usage counter
// ---------------------------------------------------------------------------

/// US Pacific DST: from the second Sunday in March to the first Sunday in
/// November. Computed rather than looked up, to avoid pulling in a timezone
/// database for one offset. The switch happens at 02:00 local, which this
/// ignores - for a request counter, a day boundary that is off by two hours
/// twice a year changes nothing.
fn is_pacific_dst(date: time::Date) -> bool {
    use time::{Month, Weekday};

    let nth_sunday = |month: Month, nth: u8| -> u8 {
        let first = time::Date::from_calendar_date(date.year(), month, 1).expect("1st is a valid day");
        let offset = (7 - first.weekday().number_days_from_sunday()) % 7;
        1 + offset as u8 + (nth - 1) * 7
    };

    match date.month() {
        Month::April
        | Month::May
        | Month::June
        | Month::July
        | Month::August
        | Month::September
        | Month::October => true,
        Month::March => date.day() >= nth_sunday(Month::March, 2),
        Month::November => date.day() < nth_sunday(Month::November, 1),
        _ => {
            let _ = Weekday::Sunday;
            false
        }
    }
}

/// Day number in Pacific time. Google's free-tier RPD quota resets at midnight
/// Pacific ("Requests per day (RPD) quotas reset at midnight Pacific time"), so
/// the counter has to roll over on the same boundary rather than on UTC.
fn pacific_day(utc: OffsetDateTime) -> i64 {
    let guess = utc.to_offset(time::UtcOffset::from_hms(-8, 0, 0).expect("valid offset"));
    let hours = if is_pacific_dst(guess.date()) { -7 } else { -8 };
    let local = utc.to_offset(time::UtcOffset::from_hms(hours, 0, 0).expect("valid offset"));
    local.date().to_julian_day() as i64
}

/// Counts every request sent today, successes and failures alike - a failed
/// request is charged against the quota exactly like a successful one.
fn record_request(now: OffsetDateTime) -> u32 {
    static USAGE: OnceLock<Mutex<(i64, u32)>> = OnceLock::new();
    let usage = USAGE.get_or_init(|| Mutex::new((pacific_day(now), 0)));

    let Ok(mut guard) = usage.lock() else { return 0 };
    let today = pacific_day(now);
    if guard.0 != today {
        *guard = (today, 0);
    }
    guard.1 += 1;
    guard.1
}

/// Time left before the whole-call budget runs out, or `None` when it is spent.
fn remaining_budget(deadline: Instant) -> Option<Duration> {
    let now = Instant::now();
    (deadline > now).then(|| deadline - now)
}

/// A backoff is only worth waiting out if the budget outlives it; otherwise the
/// wait ends in the same failure, just later.
fn backoff_fits(deadline: Instant, backoff: Duration) -> bool {
    remaining_budget(deadline).is_some_and(|left| left > backoff)
}

/// Per-call context. Carried on the provider rather than added to
/// `AiProvider::generate`, so a future provider (Ollama) is not forced to know
/// that an operation registry exists.
#[derive(Clone, Default)]
pub struct CallContext {
    pub cancel: super::cancel::CancelToken,
}

impl CallContext {
    pub fn new(cancel: super::cancel::CancelToken) -> Self {
        Self { cancel }
    }
}

pub struct GeminiProvider {
    context: CallContext,
}

impl GeminiProvider {
    pub fn new(context: CallContext) -> Self {
        Self { context }
    }

    /// For calls with no cancel button behind them; the token never fires.
    pub fn detached() -> Self {
        Self { context: CallContext::default() }
    }
}

#[async_trait::async_trait]
impl AiProvider for GeminiProvider {
    async fn generate(&self, prompt: String) -> Result<String, AiEngineError> {
        call_gemini(prompt, &self.context).await
    }
}

/// Runs `future` but abandons it the moment the operation is cancelled. Dropping
/// an in-flight `send()` tears the connection down immediately; polling a flag
/// between attempts would leave "Przerwij" doing nothing for up to
/// REQUEST_TIMEOUT, and a button that appears dead is worse than no button - the
/// user clicks it repeatedly and concludes the app has hung.
async fn race_cancel<T>(
    ctx: &CallContext,
    future: impl std::future::Future<Output = T>,
) -> Result<T, AiEngineError> {
    tokio::select! {
        // Cancellation is checked first so an already-cancelled operation never
        // starts another attempt.
        biased;
        _ = ctx.cancel.cancelled() => Err(AiEngineError::Cancelled),
        value = future => Ok(value),
    }
}

async fn call_gemini(prompt: String, ctx: &CallContext) -> Result<String, AiEngineError> {
    let api_key = crate::keychain::get_gemini_api_key().map_err(|_| AiEngineError::MissingApiKey)?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        GEMINI_MODEL
    );

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| AiEngineError::ClientBuildFailed(e.to_string()))?;

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: prompt }],
        }],
    };

    const MAX_RETRIES: u32 = 3;
    let mut last_error: Option<AiEngineError> = None;
    // Status and body length of the previous attempt; an identical pair means a
    // deterministic answer rather than a transient fault.
    let mut previous_signature: Option<(u16, usize)> = None;

    let started = Instant::now();
    let deadline = started + TOTAL_BUDGET;
    let budget_error = || AiEngineError::TimeBudgetExceeded { seconds: TOTAL_BUDGET.as_secs() };

    for attempt in 0..MAX_RETRIES {
        let Some(remaining) = remaining_budget(deadline) else {
            return Err(last_error.unwrap_or_else(budget_error));
        };

        // An async block does not run until it is first polled, and the first
        // poll is where reqwest hands the request to the transport. Flipping the
        // flag inside it therefore means exactly "this request left the machine".
        let sent = std::sync::atomic::AtomicBool::new(false);
        let request = async {
            sent.store(true, std::sync::atomic::Ordering::Release);
            client
                .post(&url)
                .header("x-goog-api-key", &api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
        };

        // Bounded by whichever runs out first: this attempt or the whole budget,
        // and abandoned outright if the user cancels.
        let attempt_outcome = race_cancel(ctx, tokio::time::timeout(remaining.min(REQUEST_TIMEOUT), request)).await;

        // Counted on sending, not on outcome: Google charges the request whether
        // it succeeds, fails or gets abandoned mid-flight. Counting earlier
        // charged cancellations that never reached the network, and at 20 a day
        // one phantom unit is 5% of the daily budget.
        if sent.load(std::sync::atomic::Ordering::Acquire) {
            let used_today = record_request(OffsetDateTime::now_utc());
            log::info!("Gemini request #{used_today} today (free-tier RPD resets at midnight Pacific)");
        }

        let res = match attempt_outcome? {
            Ok(Ok(res)) => res,
            Ok(Err(e)) => {
                // Transport faults used to leave the loop immediately, so
                // MAX_RETRIES guarded the rarest case (5xx) and not the most
                // common one - a link dropping for a second.
                let facts = transport_facts(&e);
                let retryable = is_retryable_transport(&facts);
                log::warn!(
                    "Gemini transport error (attempt {}): timeout={} connect={} builder={} retryable={}",
                    attempt + 1,
                    facts.is_timeout,
                    facts.is_connect,
                    facts.is_builder,
                    retryable
                );
                last_error = Some(AiEngineError::ConnectionFailed(e.to_string()));

                if !retryable {
                    return Err(last_error.unwrap());
                }
                let backoff = Duration::from_secs(2u64.pow(attempt + 1));
                if attempt + 1 == MAX_RETRIES || !backoff_fits(deadline, backoff) {
                    return Err(last_error.unwrap());
                }
                race_cancel(ctx, tokio::time::sleep(backoff)).await?;
                continue;
            }
            Err(_elapsed) => {
                // The budget expiring and a single slow request look the same
                // here, so the remaining budget decides which one it was.
                log::warn!("Gemini request timed out (attempt {})", attempt + 1);
                if remaining_budget(deadline).is_none() {
                    return Err(budget_error());
                }
                last_error = Some(AiEngineError::ConnectionFailed(
                    "przekroczono czas oczekiwania na odpowiedź".to_string(),
                ));
                let backoff = Duration::from_secs(2u64.pow(attempt + 1));
                if attempt + 1 == MAX_RETRIES || !backoff_fits(deadline, backoff) {
                    return Err(last_error.unwrap());
                }
                race_cancel(ctx, tokio::time::sleep(backoff)).await?;
                continue;
            }
        };

        let status = res.status();

        if status.is_success() {
            log::info!(
                "Gemini call succeeded in {} ms (attempt {})",
                started.elapsed().as_millis(),
                attempt + 1
            );
            let parsed: GeminiResponse = res
                .json()
                .await
                .map_err(|e| AiEngineError::ResponseParseFailed(e.to_string()))?;

            return parsed
                .candidates
                .first()
                .and_then(|c| c.content.parts.first())
                .map(|p| p.text.trim().to_string())
                .ok_or(AiEngineError::EmptyResponse);
        }

        let code = status.as_u16();
        let text = res.text().await.unwrap_or_default();

        // Prefer Google's RESOURCE_EXHAUSTED and retryDelay over raw JSON.
        let parsed_error: Option<GeminiErrorWrapper> = serde_json::from_str(&text).ok();

        let api_status = parsed_error
            .as_ref()
            .and_then(|w| w.error.status.as_deref())
            .unwrap_or("unknown");
        let api_code = parsed_error.as_ref().and_then(|w| w.error.code);
        let quota_id = parsed_error
            .as_ref()
            .and_then(|w| w.error.details.as_ref())
            .and_then(|details| {
                details
                    .iter()
                    .filter_map(|d| d.violations.as_ref())
                    .flatten()
                    .find_map(|v| v.quota_id.as_deref())
            });
        let scope = quota_scope(quota_id);

        // Identifiers and lengths only - never the message, the body or a URL.
        // `quotaId` and `error.code` are Google metric names, not user data, and
        // body_len alone could not tell an outage apart from an exhausted quota.
        log::warn!(
            "Gemini API {code} (attempt {}): body_len={} api_status={api_status} api_code={} quota_id={} scope={scope:?}",
            attempt + 1,
            text.len(),
            api_code.map_or_else(|| "-".to_string(), |c| c.to_string()),
            quota_id.unwrap_or("-")
        );

        let signature = (code, text.len());
        let repeated = previous_signature == Some(signature);
        previous_signature = Some(signature);
        if repeated {
            log::warn!("Identical response repeated ({code}, {} B); treating as deterministic", text.len());
        }

        let is_resource_exhausted = api_status == "RESOURCE_EXHAUSTED";
        let suggested_retry_secs = parsed_error
            .as_ref()
            .and_then(|w| w.error.details.as_ref())
            .and_then(|details| details.iter().find_map(|d| d.retry_delay.as_deref()))
            .and_then(|s| s.trim_end_matches('s').parse::<f64>().ok());

        let is_retryable = should_retry(code, attempt, scope, repeated);

        last_error = Some(if is_resource_exhausted || code == 429 {
            match scope {
                QuotaScope::PerDay => AiEngineError::DailyQuotaExhausted { limit: FREE_TIER_DAILY_LIMIT },
                _ => AiEngineError::RateLimitExceeded,
            }
        } else if status.is_client_error() {
            // 4xx: retrying cannot help, so do not imply it will.
            AiEngineError::ApiClientError {
                status: code,
                message: super::client_error_message(code),
            }
        } else {
            AiEngineError::ApiServerError {
                status: code,
                attempts: attempt + 1,
            }
        });

        if is_retryable && attempt + 1 < MAX_RETRIES {
            let backoff_secs = suggested_retry_secs
                .map(|s| s.ceil() as u64)
                .unwrap_or_else(|| 2u64.pow(attempt + 1)); // fallback: 2s, 4s, 8s
            let backoff = Duration::from_secs(backoff_secs);

            // Google's retryDelay is respected as before, but not beyond the
            // budget: when it asks for 60 s and 20 s are left, waiting only to
            // report a failure wastes the user's time. `last_error` already holds
            // RateLimitExceeded here, which says more than a generic timeout.
            if backoff_fits(deadline, backoff) {
                race_cancel(ctx, tokio::time::sleep(backoff)).await?;
                continue;
            }
            log::warn!("Budget exhausted before a {backoff_secs}s backoff; giving up");
            break;
        }

        break;
    }

    // RateLimitExceeded is its own variant, so no string matching is needed.
    // The unwrap_or is defensive: the loop always sets last_error before break.
    Err(last_error.unwrap_or(AiEngineError::ApiServerError {
        status: 0,
        attempts: MAX_RETRIES,
    }))
}

#[cfg(test)]
mod quota_tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn a_deterministic_503_is_not_retried_three_times() {
        // First failure: one more attempt is reasonable.
        assert!(should_retry(503, 0, QuotaScope::Unknown, false));
        // Second attempt returned the same status and the same body length -
        // that is an answer, not a blip, and a third attempt costs quota.
        assert!(!should_retry(503, 1, QuotaScope::Unknown, true));
        // Even without the repeat signal, a third attempt is not allowed.
        assert!(!should_retry(503, 1, QuotaScope::Unknown, false));
    }

    #[test]
    fn a_repeated_response_stops_the_loop_whatever_the_status() {
        // The heuristic is not tied to 503; any byte-identical repeat ends it.
        assert!(!should_retry(429, 0, QuotaScope::PerMinute, true));
        assert!(!should_retry(503, 0, QuotaScope::Unknown, true));
    }

    #[test]
    fn an_exhausted_daily_quota_is_never_retried() {
        // Every attempt is charged, so retrying spends quota on a certain failure.
        assert!(!should_retry(429, 0, QuotaScope::PerDay, false));
        // A per-minute cap does pass, because waiting genuinely helps.
        assert!(should_retry(429, 0, QuotaScope::PerMinute, false));
        assert!(should_retry(429, 0, QuotaScope::Unknown, false));
    }

    #[test]
    fn other_server_errors_keep_the_previous_behaviour() {
        assert!(!should_retry(500, 0, QuotaScope::Unknown, false));
        assert!(!should_retry(502, 0, QuotaScope::Unknown, false));
        assert!(!should_retry(400, 0, QuotaScope::Unknown, false));
    }

    #[test]
    fn quota_scope_reads_googles_identifier() {
        assert_eq!(
            quota_scope(Some("GenerateRequestsPerDayPerProjectPerModel-FreeTier")),
            QuotaScope::PerDay
        );
        assert_eq!(
            quota_scope(Some("GenerateRequestsPerMinutePerProjectPerModel-FreeTier")),
            QuotaScope::PerMinute
        );
        assert_eq!(quota_scope(None), QuotaScope::Unknown);
    }

    #[test]
    fn daily_and_per_minute_limits_say_different_things() {
        let per_minute = AiEngineError::RateLimitExceeded.to_string();
        let per_day = AiEngineError::DailyQuotaExhausted { limit: FREE_TIER_DAILY_LIMIT }.to_string();

        assert_ne!(per_minute, per_day);
        assert!(per_minute.contains("za chwilę"));
        // "Try again in a moment" would be false: the next success is tomorrow.
        assert!(!per_day.contains("za chwilę"));
        assert!(per_day.contains("dobę") && per_day.contains("północy"));
    }

    #[test]
    fn usage_counts_failed_requests_too() {
        let now = datetime!(2026-07-29 18:00 UTC);
        let first = record_request(now);
        let second = record_request(now);
        assert_eq!(second, first + 1, "nieudane żądanie też zużywa kwotę");
    }

    #[test]
    fn the_counter_rolls_over_at_pacific_midnight_not_utc_midnight() {
        // 07:30 UTC on 30 July is still 29 July in Pacific (PDT, UTC-7).
        let late_pacific = datetime!(2026-07-30 06:30 UTC);
        let next_pacific_day = datetime!(2026-07-30 07:30 UTC);
        assert_eq!(pacific_day(late_pacific), pacific_day(datetime!(2026-07-29 20:00 UTC)));
        assert_eq!(pacific_day(next_pacific_day), pacific_day(late_pacific) + 1);
    }

    #[test]
    fn pacific_dst_boundaries_follow_the_us_rules() {
        use time::macros::date;
        // Second Sunday in March 2026 is the 8th; first Sunday in November is the 1st.
        assert!(!is_pacific_dst(date!(2026 - 03 - 07)));
        assert!(is_pacific_dst(date!(2026 - 03 - 08)));
        assert!(is_pacific_dst(date!(2026 - 10 - 31)));
        assert!(!is_pacific_dst(date!(2026 - 11 - 01)));
        assert!(!is_pacific_dst(date!(2026 - 01 - 15)));
        assert!(is_pacific_dst(date!(2026 - 07 - 29)));
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use crate::ai_engine::cancel::CancelToken;

    /// `race_cancel` is the same wrapper the real request goes through, so a
    /// long sleep stands in for a request that would otherwise run to timeout.
    #[tokio::test]
    async fn cancelling_abandons_the_request_instead_of_waiting_for_the_timeout() {
        let token = CancelToken::default();
        let ctx = CallContext::new(token.clone());
        token.cancel();

        let started = Instant::now();
        let result = race_cancel(&ctx, tokio::time::sleep(REQUEST_TIMEOUT)).await;

        assert!(matches!(result, Err(AiEngineError::Cancelled)));
        assert!(
            started.elapsed() < Duration::from_millis(200),
            "anulowanie czekało na timeout żądania"
        );
    }

    #[tokio::test]
    async fn cancelling_during_a_backoff_stops_the_sleep_immediately() {
        let token = CancelToken::default();
        let ctx = CallContext::new(token.clone());

        let waiter = tokio::spawn(async move {
            let started = Instant::now();
            let result = race_cancel(&ctx, tokio::time::sleep(Duration::from_secs(60))).await;
            (result.is_err(), started.elapsed())
        });

        tokio::task::yield_now().await;
        token.cancel();

        let (was_cancelled, elapsed) = tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("backoff nie został przerwany")
            .unwrap();
        assert!(was_cancelled);
        assert!(elapsed < Duration::from_secs(1), "sleep dobiegł końca mimo anulowania");
    }

    #[tokio::test]
    async fn cancelling_before_the_request_is_sent_does_not_count_a_unit() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let token = CancelToken::default();
        let ctx = CallContext::new(token.clone());
        token.cancel();

        // Mirrors the production shape: the flag flips on the first poll, which
        // is where the request would reach the transport.
        let sent = AtomicBool::new(false);
        let result = race_cancel(&ctx, async {
            sent.store(true, Ordering::Release);
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await;

        assert!(matches!(result, Err(AiEngineError::Cancelled)));
        assert!(
            !sent.load(Ordering::Acquire),
            "żądanie nie wyszło, więc kwota nie może zostać naliczona"
        );
    }

    #[tokio::test]
    async fn a_request_that_is_actually_sent_is_counted() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let ctx = CallContext::default();
        let sent = AtomicBool::new(false);
        race_cancel(&ctx, async {
            sent.store(true, Ordering::Release);
        })
        .await
        .unwrap();

        assert!(sent.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn an_uncancelled_operation_runs_to_completion() {
        let ctx = CallContext::default();
        let value = race_cancel(&ctx, async { 42 }).await.unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn cancellation_message_is_the_frontend_contract() {
        // The IPC layer stringifies errors, so this exact sentence is what
        // utils/aiOperations.ts matches on to return to idle instead of showing
        // an error. Changing it here means changing it there.
        assert_eq!(AiEngineError::Cancelled.to_string(), "Analiza została przerwana.");
    }

    #[test]
    fn cancellation_is_a_different_variant_than_the_time_budget() {
        let cancelled = AiEngineError::Cancelled;
        let timed_out = AiEngineError::TimeBudgetExceeded { seconds: TOTAL_BUDGET.as_secs() };

        assert!(!matches!(cancelled, AiEngineError::TimeBudgetExceeded { .. }));
        assert_ne!(cancelled.to_string(), timed_out.to_string());
        // Cancellation is a user decision, so it must not read like a failure.
        assert!(!cancelled.to_string().to_lowercase().contains("błąd"));
        assert!(timed_out.to_string().contains("limit czasu"));
    }
}

#[cfg(test)]
mod retry_policy_tests {
    use super::*;

    fn facts() -> TransportFacts {
        TransportFacts::default()
    }

    #[test]
    fn transient_faults_are_retried() {
        assert!(is_retryable_transport(&TransportFacts { is_timeout: true, ..facts() }));
        assert!(is_retryable_transport(&TransportFacts { is_connect: true, ..facts() }));
        assert!(is_retryable_transport(&TransportFacts { is_request: true, ..facts() }));
        assert!(is_retryable_transport(&TransportFacts { is_body: true, ..facts() }));
    }

    #[test]
    fn certain_failures_are_not_retried() {
        // Waiting three times for the same guaranteed answer helps nobody.
        assert!(!is_retryable_transport(&TransportFacts { is_builder: true, ..facts() }));
        assert!(!is_retryable_transport(&TransportFacts { is_redirect: true, ..facts() }));
        assert!(!is_retryable_transport(&TransportFacts { is_decode: true, ..facts() }));
    }

    #[test]
    fn tls_failures_are_not_retried_even_though_they_look_like_connect_errors() {
        let tls = TransportFacts {
            is_connect: true,
            chain: "error sending request; invalid peer certificate: unknown issuer".to_string(),
            ..facts()
        };
        assert!(!is_retryable_transport(&tls));
    }

    #[test]
    fn unclassified_errors_fail_fast() {
        assert!(!is_retryable_transport(&facts()));
    }

    #[test]
    fn budget_reports_exhaustion() {
        let past = Instant::now() - Duration::from_secs(1);
        assert!(remaining_budget(past).is_none());
        assert!(remaining_budget(Instant::now() + Duration::from_secs(10)).is_some());
    }

    #[test]
    fn backoff_is_skipped_when_the_budget_would_not_survive_it() {
        let deadline = Instant::now() + Duration::from_secs(5);
        // Google asking for 60 s with 5 s left: report now instead of waiting.
        assert!(!backoff_fits(deadline, Duration::from_secs(60)));
        assert!(backoff_fits(deadline, Duration::from_secs(2)));
        assert!(!backoff_fits(Instant::now() - Duration::from_secs(1), Duration::from_secs(1)));
    }

    #[test]
    fn budget_covers_every_attempt_and_backoff() {
        // Three 45 s attempts plus 2 s and 4 s backoffs would reach 141 s; the
        // budget is what actually caps the wait.
        assert!(TOTAL_BUDGET < REQUEST_TIMEOUT * 3);
        assert!(TOTAL_BUDGET > REQUEST_TIMEOUT, "budżet musi mieścić choć jedną pełną próbę");
    }
}
