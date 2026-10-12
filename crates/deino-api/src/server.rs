//! [`RunningServer`]: the router served on `127.0.0.1` until stopped.
//!
//! [`bind_address`] is the only address the API ever binds, so the server
//! is unreachable from the network whatever the port: no `0.0.0.0`, no
//! LAN address, no IPv6 wildcard.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::{ApiServices, ApiState, router};

/// How long a stop waits for requests in flight.
const STOP_GRACE: Duration = Duration::from_secs(5);

/// The address the API listens on for `port`: always loopback.
pub fn bind_address(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

/// Why the server did not start.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The port is taken or cannot be bound.
    #[error("could not listen on 127.0.0.1:{port}: {source}")]
    Bind {
        /// The port asked for.
        port: u16,
        /// What the OS said.
        source: std::io::Error,
    },
}

/// The API, listening.
pub struct RunningServer {
    address: SocketAddr,
    cancel: CancellationToken,
    tracker: TaskTracker,
}

impl RunningServer {
    /// Binds `127.0.0.1:port` (0 picks a free port) and serves the API
    /// over `services` until [`RunningServer::stop`].
    pub async fn start(services: Arc<ApiServices>, port: u16) -> Result<Self, ServeError> {
        let bind = |source| ServeError::Bind { port, source };
        let listener = TcpListener::bind(bind_address(port)).await.map_err(bind)?;
        let address = listener.local_addr().map_err(bind)?;
        let state = ApiState {
            services,
            port: Some(address.port()),
        };
        let app = router(state);
        let cancel = CancellationToken::new();
        let tracker = TaskTracker::new();
        let shutdown = cancel.clone().cancelled_owned();
        tracker.spawn(async move {
            let served = axum::serve(listener, app).with_graceful_shutdown(shutdown);
            if let Err(error) = served.await {
                log::warn!(target: "api", "the API server stopped: {error}");
            }
        });
        tracker.close();
        log::info!(target: "api", "serving the local API on {address}");
        Ok(Self {
            address,
            cancel,
            tracker,
        })
    }

    /// Where it listens.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Stops listening and waits (briefly) for requests in flight.
    pub async fn stop(self) {
        self.cancel.cancel();
        if tokio::time::timeout(STOP_GRACE, self.tracker.wait())
            .await
            .is_err()
        {
            log::warn!(target: "api", "the API server did not stop within {STOP_GRACE:?}");
        }
        log::info!(target: "api", "stopped the local API on {}", self.address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_port_binds_loopback_only() {
        for port in [0, 1024, 47_810, u16::MAX] {
            let address = bind_address(port);
            assert!(address.ip().is_loopback(), "{address}");
            assert!(!address.ip().is_unspecified(), "{address}");
            assert_eq!(address.port(), port);
        }
    }
}
