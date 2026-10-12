//! The real server: it binds loopback only, answers over TCP, and starts
//! and stops with its setting.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

mod support;

use std::net::TcpListener as StdListener;
use std::sync::Arc;

use deino_api::{ApiController, RunningServer};
use deino_core::settings::{
    ApiSettings, FakeSettingsStore, RecordedSettingsEvents, SettingsService,
};
use support::Harness;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// A port nothing listens on right now.
fn free_port() -> u16 {
    StdListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// The status line of a raw `GET path` sent to `port`.
async fn status_line(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).await.unwrap();
    reply.lines().next().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn the_server_binds_only_loopback_and_answers_over_tcp() {
    let api = Harness::new().await;

    let server = RunningServer::start(api.services.clone(), 0).await.unwrap();
    let address = server.address();

    assert!(address.ip().is_loopback(), "{address}");
    assert!(!address.ip().is_unspecified(), "{address}");
    assert_eq!(
        status_line(address.port(), "/v1/meetings").await,
        "HTTP/1.1 401 Unauthorized"
    );
    server.stop().await;
    assert!(
        TcpStream::connect(address).await.is_err(),
        "still listening after stop"
    );
}

async fn controller(api: &Harness) -> ApiController {
    let settings = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    )
    .await
    .unwrap();
    ApiController::new(api.services.clone(), Arc::new(settings))
}

#[tokio::test]
async fn the_server_follows_its_setting() {
    let api = Harness::new().await;
    let controller = controller(&api).await;
    assert_eq!(
        controller.reconcile().await.base_url,
        None,
        "off by default"
    );
    let port = free_port();

    let on = controller
        .configure(ApiSettings {
            enabled: true,
            port,
        })
        .await
        .unwrap();
    let base = on.base_url.clone().unwrap();
    assert_eq!(base, format!("http://127.0.0.1:{port}/v1"));
    assert!(status_line(port, "/v1/tags").await.contains("401"));

    let moved_to = free_port();
    let moved = controller
        .configure(ApiSettings {
            enabled: true,
            port: moved_to,
        })
        .await
        .unwrap();
    assert_eq!(
        moved.base_url,
        Some(format!("http://127.0.0.1:{moved_to}/v1"))
    );
    assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());

    let off = controller
        .configure(ApiSettings {
            enabled: false,
            port: moved_to,
        })
        .await
        .unwrap();
    assert_eq!((off.base_url, off.error), (None, None));
    assert!(TcpStream::connect(("127.0.0.1", moved_to)).await.is_err());
}

#[tokio::test]
async fn a_taken_port_is_reported_not_fatal() {
    let api = Harness::new().await;
    let controller = controller(&api).await;
    let taken = StdListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();

    let status = controller
        .configure(ApiSettings {
            enabled: true,
            port,
        })
        .await
        .unwrap();

    assert_eq!(status.base_url, None);
    assert!(status.error.unwrap().contains(&port.to_string()));
    let refused = controller
        .configure(ApiSettings {
            enabled: true,
            port: 80,
        })
        .await;
    assert!(
        refused.is_err(),
        "a privileged port is refused by the settings"
    );
}
