//! [`Failure`]: a provider's non-2xx answer, kept whole for its classifier.
//!
//! Providers signal "slow down" and "sign in again" in their own ways: 429
//! with `Retry-After` (most), HTTP 400 with `RATELIMITED` in the body
//! (Linear), 403 with `REQUEST_LIMIT_EXCEEDED` (Salesforce), a body error
//! code (Zoho) or an epoch reset header (ClickUp). So the status, headers and
//! (capped) body are all kept, and each provider's
//! [`Integration::classify`](crate::Integration::classify) decides;
//! [`standard_classification`] is the RFC behaviour most providers share.

use std::fmt;
use std::time::Duration;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use deino_net::{Headers, Response, Status};

use crate::ProviderError;

/// The most of a failed answer's body that is kept: enough for an error
/// code, never a whole document.
pub const MAX_FAILURE_BODY: usize = 64 * 1024;

/// A non-2xx answer: status, headers and at most [`MAX_FAILURE_BODY`] bytes
/// of body. Bodies may name the user's data, so `Debug` prints the status
/// only and nothing here is ever logged.
#[derive(Clone)]
pub struct Failure {
    /// The status.
    pub status: Status,
    headers: Headers,
    body: Bytes,
}

impl Failure {
    /// A failure from its parts (tests and recorded fixtures).
    pub fn new(status: Status, headers: Headers, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers,
            body: body.into(),
        }
    }

    /// Reads `response`; a body that cannot be read, or is too large, is
    /// kept empty, since the status still says what happened.
    pub(crate) async fn read(response: Response) -> Self {
        let status = response.status;
        let headers = response.headers.clone();
        let body = response.bytes(MAX_FAILURE_BODY).await.unwrap_or_default();
        Self {
            status,
            headers,
            body,
        }
    }

    /// The value of the header `name`, if present.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    /// The (capped) body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Whether the body contains `needle`, for providers that put an error
    /// code in it (`RATELIMITED`, `TOO_MANY_REQUESTS`).
    pub fn body_contains(&self, needle: &str) -> bool {
        let needle = needle.as_bytes();
        !needle.is_empty()
            && self
                .body
                .windows(needle.len())
                .any(|window| window == needle)
    }

    /// How long `Retry-After` asks to wait: delta-seconds, or an HTTP date
    /// measured from `now` (RFC 9110 §10.2.3).
    pub fn retry_after(&self, now: DateTime<Utc>) -> Option<Duration> {
        let value = self.header("retry-after")?.trim();
        if let Ok(seconds) = value.parse::<u64>() {
            return Some(Duration::from_secs(seconds));
        }
        let at = DateTime::parse_from_rfc2822(value).ok()?;
        (at.with_timezone(&Utc) - now).to_std().ok()
    }
}

impl fmt::Debug for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Failure")
            .field("status", &self.status.as_u16())
            .finish_non_exhaustive()
    }
}

/// The classification most providers share: 401 needs a new sign-in, 429 is
/// rate limiting (honouring `Retry-After`), 5xx and 408 are a temporary
/// outage, and anything else is a refusal.
pub fn standard_classification(failure: &Failure, now: DateTime<Utc>) -> ProviderError {
    match failure.status.as_u16() {
        401 => ProviderError::Unauthorized,
        429 => ProviderError::RateLimited {
            retry_after: failure.retry_after(now),
        },
        408 | 500..=599 => ProviderError::Unavailable,
        status => ProviderError::Rejected {
            reason: format!("the request was refused ({status})"),
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap()
    }

    fn failure(status: u16, headers: &[(&str, &str)], body: &'static str) -> Failure {
        let headers = headers.iter().copied().collect();
        Failure::new(Status::new(status), headers, body)
    }

    #[test]
    fn retry_after_reads_seconds_and_http_dates() {
        let seconds = failure(429, &[("retry-after", " 30 ")], "");
        let date = failure(429, &[("retry-after", "Sun, 11 Oct 2026 09:01:00 GMT")], "");
        let past = failure(429, &[("retry-after", "Sun, 11 Oct 2026 08:00:00 GMT")], "");

        assert_eq!(seconds.retry_after(now()), Some(Duration::from_secs(30)));
        assert_eq!(date.retry_after(now()), Some(Duration::from_mins(1)));
        assert_eq!(past.retry_after(now()), None);
        assert_eq!(failure(429, &[], "").retry_after(now()), None);
    }

    #[test]
    fn the_standard_classification_follows_the_status() {
        let classify = |status| standard_classification(&failure(status, &[], ""), now());

        assert!(matches!(classify(401), ProviderError::Unauthorized));
        assert!(matches!(
            classify(429),
            ProviderError::RateLimited { retry_after: None }
        ));
        assert!(matches!(classify(503), ProviderError::Unavailable));
        assert!(matches!(classify(408), ProviderError::Unavailable));
        assert!(matches!(classify(404), ProviderError::Rejected { .. }));
    }

    #[test]
    fn body_codes_can_be_found_without_decoding() {
        let linear = failure(
            400,
            &[],
            r#"{"errors":[{"extensions":{"code":"RATELIMITED"}}]}"#,
        );

        assert!(linear.body_contains("RATELIMITED"));
        assert!(!linear.body_contains("FORBIDDEN"));
        assert!(!linear.body_contains(""));
    }

    #[test]
    fn debug_output_shows_the_status_but_not_the_body() {
        let printed = format!("{:?}", failure(400, &[], "secret meeting title"));
        assert!(printed.contains("400"));
        assert!(!printed.contains("secret"));
    }
}
