use super::*;
use crate::testing::browser_get;

const SPEC: RedirectSpec = RedirectSpec {
    host: LoopbackHost::Ipv4,
    port: PortRule::Any,
    path: "/callback",
};

/// Long enough never to fire in a passing test.
const PATIENCE: Duration = Duration::from_secs(30);

fn callback(listener: &LoopbackListener, query: &str) -> Url {
    let mut url = listener.redirect_uri().clone();
    url.set_query(Some(query));
    url
}

#[tokio::test]
async fn binds_an_ephemeral_loopback_port_and_names_it_in_the_redirect() {
    let listener = LoopbackListener::bind(SPEC, None).await.unwrap();

    let uri = listener.redirect_uri();
    assert_eq!(uri.host_str(), Some("127.0.0.1"));
    assert_eq!(uri.path(), "/callback");
    assert_ne!(uri.port(), Some(0));
}

#[tokio::test]
async fn returns_the_code_of_the_matching_state_and_ignores_strays() {
    let listener = LoopbackListener::bind(SPEC, None).await.unwrap();
    let favicon = listener.redirect_uri().join("/favicon.ico").unwrap();
    let forged = callback(&listener, "code=evil&state=other");
    let real = callback(&listener, "code=c-1&state=s-1");
    let cancel = CancellationToken::new();

    let browser = async {
        let missing = browser_get(&favicon).await.unwrap();
        let refused = browser_get(&forged).await.unwrap();
        let done = browser_get(&real).await.unwrap();
        (missing, refused, done)
    };
    let (code, (missing, refused, done)) =
        tokio::join!(listener.wait("s-1", &cancel, PATIENCE), browser);

    assert_eq!(code.unwrap().as_deref(), Some("c-1"));
    assert!(missing.starts_with("HTTP/1.1 404"));
    assert!(refused.starts_with("HTTP/1.1 400"));
    assert!(done.starts_with("HTTP/1.1 200") && done.contains("close this tab"));
}

#[tokio::test]
async fn a_declined_sign_in_ends_the_wait_as_access_denied() {
    let listener = LoopbackListener::bind(SPEC, None).await.unwrap();
    let declined = callback(&listener, "error=access_denied&state=s-1");
    let cancel = CancellationToken::new();

    let (outcome, page) = tokio::join!(
        listener.wait("s-1", &cancel, PATIENCE),
        browser_get(&declined)
    );

    assert!(matches!(outcome, Err(OAuthError::AccessDenied)));
    assert!(page.unwrap().starts_with("HTTP/1.1 400"));
}

#[tokio::test]
async fn cancelling_stops_the_wait_without_an_error() {
    let listener = LoopbackListener::bind(SPEC, None).await.unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();

    let outcome = listener.wait("s-1", &cancel, PATIENCE).await;

    assert!(matches!(outcome, Ok(None)));
}

#[tokio::test]
async fn a_taken_fixed_port_is_reported_by_number() {
    let taken = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = taken.local_addr().unwrap().port();
    let fixed = RedirectSpec {
        port: PortRule::Fixed(1),
        host: LoopbackHost::Localhost,
        ..SPEC
    };

    let error = LoopbackListener::bind(fixed, Some(port)).await.unwrap_err();

    assert!(matches!(error, OAuthError::PortInUse(reported) if reported == port));
}

#[tokio::test]
async fn localhost_redirects_name_localhost() {
    let spec = RedirectSpec {
        host: LoopbackHost::Localhost,
        ..SPEC
    };

    let listener = LoopbackListener::bind(spec, None).await.unwrap();

    assert_eq!(listener.redirect_uri().host_str(), Some("localhost"));
}
