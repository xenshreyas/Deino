//! Turning a provider's failed HTTP response into an [`LlmError`].
//!
//! Shared by every HTTP provider: status codes map to the error taxonomy,
//! `retry-after` (seconds) or `retry-after-ms` becomes `retry_after`, and
//! the provider's own error text is kept short. Bodies that are not JSON
//! are never quoted: they could echo the request.

use std::time::Duration;

use serde_json::Value;

use crate::{HttpError, HttpResponse, LlmError, ProviderId};

/// The longest provider error text an [`LlmError`] keeps.
const MAX_MESSAGE_CHARS: usize = 200;

/// Reads a non-2xx response and classifies it.
pub(crate) async fn error_from(provider: ProviderId, response: HttpResponse) -> LlmError {
    let status = response.status;
    let retry_after = retry_after(&response);
    let body = response.bytes().await.unwrap_or_default();
    classify(
        provider,
        status,
        retry_after,
        provider_message(&body, status),
    )
}

/// Maps a status (and the provider's message) to the error taxonomy.
pub(crate) fn classify(
    provider: ProviderId,
    status: u16,
    retry_after: Option<Duration>,
    message: String,
) -> LlmError {
    match status {
        401 | 403 => LlmError::Auth(provider),
        429 => LlmError::RateLimited {
            provider,
            retry_after,
        },
        503 | 529 => LlmError::Overloaded {
            provider,
            retry_after,
        },
        413 => LlmError::ContextTooLong(provider),
        400..=499 if mentions_context_length(&message) => LlmError::ContextTooLong(provider),
        400..=499 => LlmError::Rejected {
            provider,
            status,
            message,
        },
        _ => LlmError::Provider {
            provider,
            status,
            message,
        },
    }
}

/// The transport failed: a network error for `provider`.
pub(crate) fn network(provider: ProviderId) -> impl Fn(HttpError) -> LlmError {
    move |source| LlmError::Network { provider, source }
}

/// The error text of a JSON error body (`{"error":{"message":..}}`,
/// `{"error":".."}` or `{"message":..}`), shortened; a generic text otherwise.
pub(crate) fn provider_message(body: &[u8], status: u16) -> String {
    let message = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| message_in(&value).map(str::to_owned));
    match message {
        Some(message) => shorten(&message),
        None => format!("HTTP {status}"),
    }
}

fn message_in(value: &Value) -> Option<&str> {
    let error = value.get("error");
    error
        .and_then(|error| error.get("message"))
        .or(error)
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
}

fn shorten(message: &str) -> String {
    let mut short: String = message.trim().chars().take(MAX_MESSAGE_CHARS).collect();
    if message.trim().chars().count() > MAX_MESSAGE_CHARS {
        short.push('…');
    }
    short
}

fn mentions_context_length(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "context length",
        "context window",
        "maximum context",
        "prompt is too long",
        "too many tokens",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

fn retry_after(response: &HttpResponse) -> Option<Duration> {
    let millis = response
        .header("retry-after-ms")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .map(|ms| ms / 1000.0);
    let seconds = millis.or_else(|| {
        response
            .header("retry-after")
            .and_then(|value| value.trim().parse::<f64>().ok())
    })?;
    Duration::try_from_secs_f64(seconds).ok()
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use futures_util::stream;

    use super::*;

    fn response(status: u16, headers: &[(&str, &str)], body: &str) -> HttpResponse {
        HttpResponse {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: stream::iter([Ok(body.as_bytes().to_vec())]).boxed(),
        }
    }

    #[tokio::test]
    async fn a_429_is_rate_limited_with_its_retry_after() {
        let error = error_from(
            ProviderId::Anthropic,
            response(
                429,
                &[("retry-after", "12")],
                r#"{"error":{"message":"slow down"}}"#,
            ),
        )
        .await;

        assert_eq!(error.retry_after(), Some(Duration::from_secs(12)));
        assert!(matches!(error, LlmError::RateLimited { .. }));
    }

    #[tokio::test]
    async fn retry_after_ms_wins_over_seconds() {
        let error = error_from(
            ProviderId::OpenAi,
            response(
                429,
                &[("retry-after-ms", "1500"), ("retry-after", "9")],
                "{}",
            ),
        )
        .await;

        assert_eq!(error.retry_after(), Some(Duration::from_millis(1500)));
    }

    #[test]
    fn statuses_map_to_the_taxonomy() {
        let kind = |status, message: &str| {
            classify(ProviderId::Groq, status, None, message.to_owned()).kind()
        };

        assert_eq!(kind(401, ""), crate::LlmErrorKind::Auth);
        assert_eq!(kind(529, ""), crate::LlmErrorKind::Overloaded);
        assert_eq!(
            kind(400, "prompt is too long: 9 > 8"),
            crate::LlmErrorKind::ContextTooLong
        );
        assert_eq!(kind(422, "bad field"), crate::LlmErrorKind::Rejected);
        assert_eq!(kind(500, "oops"), crate::LlmErrorKind::Provider);
    }

    #[test]
    fn messages_come_from_json_only_and_are_shortened() {
        let long = format!(r#"{{"error":"{}"}}"#, "x".repeat(500));

        assert_eq!(provider_message(br#"{"message":"bad"}"#, 400), "bad");
        assert_eq!(
            provider_message(b"<html>echo of the prompt</html>", 502),
            "HTTP 502"
        );
        assert_eq!(provider_message(long.as_bytes(), 400).chars().count(), 201);
    }
}
