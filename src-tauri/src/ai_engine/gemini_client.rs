//! Raw HTTP client for the Gemini API: request/response shape, retry with
//! backoff, RESOURCE_EXHAUSTED detection. `GeminiProvider` is currently the
//! only `AiProvider` implementation (see `mod.rs`); no other submodule should
//! call `call_gemini` directly rather than through the trait.

use super::{AiEngineError, AiProvider};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const GEMINI_MODEL: &str = "gemini-3.5-flash";

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
struct GeminiErrorDetail {
    #[serde(rename = "retryDelay")]
    retry_delay: Option<String>,
}

#[derive(Deserialize)]
struct GeminiErrorBody {
    status: Option<String>,
    details: Option<Vec<GeminiErrorDetail>>,
}

#[derive(Deserialize)]
struct GeminiErrorWrapper {
    error: GeminiErrorBody,
}

pub struct GeminiProvider;

#[async_trait::async_trait]
impl AiProvider for GeminiProvider {
    async fn generate(&self, prompt: String) -> Result<String, AiEngineError> {
        call_gemini(prompt).await
    }
}

async fn call_gemini(prompt: String) -> Result<String, AiEngineError> {
    let api_key = crate::keychain::get_gemini_api_key().map_err(|_| AiEngineError::MissingApiKey)?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        GEMINI_MODEL
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| AiEngineError::ClientBuildFailed(e.to_string()))?;

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: prompt }],
        }],
    };

    const MAX_RETRIES: u32 = 3;
    let mut last_error: Option<AiEngineError> = None;

    for attempt in 0..MAX_RETRIES {
        let res = client
            .post(&url)
            .header("x-goog-api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiEngineError::ConnectionFailed(e.to_string()))?;

        let status = res.status();

        if status.is_success() {
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
        let is_retryable = code == 503 || code == 429;
        let text = res.text().await.unwrap_or_default();

        // Google's raw response is log-only; the user gets a message matched
        // to the status class (see AiEngineError).
        eprintln!("Gemini API {code} (attempt {}): {text}", attempt + 1);

        // Prefer Google's RESOURCE_EXHAUSTED and retryDelay over raw JSON.
        let parsed_error: Option<GeminiErrorWrapper> = serde_json::from_str(&text).ok();
        let is_resource_exhausted = parsed_error
            .as_ref()
            .and_then(|w| w.error.status.as_deref())
            .map(|s| s == "RESOURCE_EXHAUSTED")
            .unwrap_or(false);
        let suggested_retry_secs = parsed_error
            .as_ref()
            .and_then(|w| w.error.details.as_ref())
            .and_then(|details| details.iter().find_map(|d| d.retry_delay.as_deref()))
            .and_then(|s| s.trim_end_matches('s').parse::<f64>().ok());

        last_error = Some(if is_resource_exhausted || code == 429 {
            AiEngineError::RateLimitExceeded
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
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            continue;
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
