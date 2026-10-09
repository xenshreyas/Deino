//! Following a redirect safely.
//!
//! The root client never follows redirects, so the scoped client can check
//! every hop's host. A hop to another origin carries only harmless headers
//! (never `authorization` or an API key header), an `https` request is never
//! downgraded to `http`, and a `303` (or a `301`/`302` after a `POST`)
//! becomes a body-less `GET`, as browsers do.

use crate::{Method, NetError, Request, Response, Status};

/// Headers a request keeps when it is redirected to another origin.
const CROSS_ORIGIN_HEADERS: [&str; 4] = ["accept", "accept-encoding", "range", "user-agent"];

/// The request to send next, or `None` when `response` is not a redirect.
pub(super) fn next(request: &Request, response: &Response) -> Result<Option<Request>, NetError> {
    if !response.status.is_redirect() {
        return Ok(None);
    }
    let location = response.header("location").ok_or(NetError::Redirect)?;
    let url = request.url.join(location).map_err(|_| NetError::Redirect)?;
    let downgrade = request.url.scheme() == "https" && url.scheme() != "https";
    if downgrade || url.scheme() != "http" && url.scheme() != "https" {
        return Err(NetError::Redirect);
    }
    let mut next = request.clone();
    if url.origin() != request.url.origin() {
        next.headers
            .retain(|name| CROSS_ORIGIN_HEADERS.contains(&name));
    }
    next.url = url;
    if becomes_get(request.method, response.status) {
        next.method = Method::Get;
        next.body = None;
        next.headers
            .retain(|name| name != "content-type" && name != "content-length");
    }
    Ok(Some(next))
}

/// Whether the redirected request is a `GET` without a body.
fn becomes_get(method: Method, status: Status) -> bool {
    status == Status::SEE_OTHER
        || method == Method::Post && matches!(status, Status::MOVED_PERMANENTLY | Status::FOUND)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redirect(status: Status, location: &str) -> Response {
        let mut response = Response::empty(status);
        response.headers.insert("location", location);
        response
    }

    fn api_request() -> Request {
        Request::post("https://huggingface.co/model/resolve/main/model.gguf")
            .unwrap()
            .header("authorization", "Bearer hf_secret")
            .header("range", "bytes=10-")
            .body("payload")
    }

    #[test]
    fn a_cross_origin_hop_drops_credentials_but_keeps_the_range() {
        let response = redirect(Status::TEMPORARY_REDIRECT, "https://cdn-lfs.hf.co/blob?sig=1");

        let next = next(&api_request(), &response).unwrap().unwrap();

        assert_eq!(next.url.host_str(), Some("cdn-lfs.hf.co"));
        assert_eq!(next.headers.get("authorization"), None);
        assert_eq!(next.headers.get("range"), Some("bytes=10-"));
        assert_eq!(next.method, Method::Post);
        assert!(next.body.is_some());
    }

    #[test]
    fn a_same_origin_hop_keeps_headers_and_resolves_relative_locations() {
        let response = redirect(Status::FOUND, "/other/path");
        let request = Request::get("https://huggingface.co/a/b")
            .unwrap()
            .header("authorization", "Bearer hf");

        let next = next(&request, &response).unwrap().unwrap();

        assert_eq!(next.url.as_str(), "https://huggingface.co/other/path");
        assert_eq!(next.headers.get("authorization"), Some("Bearer hf"));
    }

    #[test]
    fn see_other_and_a_found_post_become_a_bodyless_get() {
        for status in [Status::SEE_OTHER, Status::FOUND] {
            let next = next(&api_request(), &redirect(status, "/done")).unwrap().unwrap();
            assert_eq!(next.method, Method::Get);
            assert_eq!(next.body, None);
        }
    }

    #[test]
    fn downgrades_missing_locations_and_other_schemes_are_refused() {
        let request = api_request();
        for location in ["http://huggingface.co/x", "file:///etc/passwd"] {
            let response = redirect(Status::FOUND, location);
            assert!(matches!(next(&request, &response), Err(NetError::Redirect)));
        }
        let bare = Response::empty(Status::FOUND);
        assert!(matches!(next(&request, &bare), Err(NetError::Redirect)));
    }

    #[test]
    fn a_non_redirect_ends_the_chain() {
        let response = Response::empty(Status::OK);
        assert!(next(&api_request(), &response).unwrap().is_none());
    }
}
