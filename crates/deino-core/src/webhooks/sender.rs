//! [`send`]: one signed `POST` through the `webhooks` scoped client
//! (ADR 0019), which checks the host, the user's switch and offline mode
//! and records the attempt in the egress log.

use deino_net::{NetClient, NetError, Request};

use super::schedule::{ATTEMPT_TIMEOUT, AttemptResult};
use super::signing::SignatureHeaders;

/// The most of an endpoint's answer Deino reads (and then ignores).
const MAX_ANSWER_BYTES: usize = 64 * 1024;

/// What one attempt sends.
#[derive(Clone, Copy, Debug)]
pub struct Outgoing<'a> {
    /// The endpoint.
    pub url: &'a str,
    /// The JSON body.
    pub body: &'a str,
    /// The signature headers.
    pub headers: &'a SignatureHeaders,
}

/// Posts `outgoing` through `client` and says how it went.
pub async fn send(client: &dyn NetClient, outgoing: Outgoing<'_>) -> AttemptResult {
    let request = match request(outgoing) {
        Ok(request) => request,
        Err(error) => return AttemptResult::Rejected(error.to_string()),
    };
    match client.send(request).await {
        Ok(response) => {
            let status = response.status.as_u16();
            // Read (a little of) the answer so the connection is reused; its
            // content does not matter.
            if response.bytes(MAX_ANSWER_BYTES).await.is_err() {
                log::debug!(target: "webhooks", "the endpoint's answer was cut short");
            }
            if (200..300).contains(&status) {
                AttemptResult::Delivered(status)
            } else {
                AttemptResult::Status(status)
            }
        }
        Err(error) if error.is_blocked() => AttemptResult::Blocked(error.to_string()),
        Err(NetError::Timeout(_)) => AttemptResult::NoAnswer(format!(
            "the endpoint did not answer within {} seconds",
            ATTEMPT_TIMEOUT.as_secs()
        )),
        Err(error) => AttemptResult::NoAnswer(error.to_string()),
    }
}

/// The signed request.
fn request(outgoing: Outgoing<'_>) -> Result<Request, NetError> {
    let mut request = Request::post(outgoing.url)?
        .header("content-type", "application/json")
        .header(
            "user-agent",
            concat!("Deino-Webhooks/", env!("CARGO_PKG_VERSION")),
        )
        .body(outgoing.body.to_owned())
        .timeout(Some(ATTEMPT_TIMEOUT));
    for (name, value) in outgoing.headers.pairs() {
        request = request.header(name, value);
    }
    Ok(request)
}
