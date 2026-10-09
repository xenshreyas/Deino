//! [`Net`] and [`ScopedClient`]: capability-style access to the network.
//!
//! The app builds one [`Net`] over the real client and hands each consumer a
//! [`ScopedClient`] for its own [`ServiceId`] (`net.scoped(ServiceId::LlmAnthropic)`),
//! never the root client. Before every request, and every redirect hop, the
//! scoped client checks that:
//!
//! 1. the host is one of the service's (the registry's, plus a custom
//!    endpoint from the settings for services that take one);
//! 2. the user has not switched the service off;
//! 3. offline mode is off, unless this is a model download the user started
//!    ([`Net::approved_download`]).
//!
//! Every attempt, allowed or not, becomes one egress log entry.

mod redirect;

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Clock, ServiceId};
use futures_util::StreamExt;

use crate::egress::{EgressEntry, EgressLog, EgressOutcome, Exchange};
use crate::{NetClient, NetError, Request, Response, ServicePolicy, registry};

/// The most redirects one request may follow.
pub const MAX_REDIRECTS: usize = 10;

/// What a [`Net`] is built from.
pub struct NetParts {
    /// The root client that actually sends (the real one, or a fake).
    pub client: Arc<dyn NetClient>,
    /// The user's network choices.
    pub policy: Arc<dyn ServicePolicy>,
    /// Where every request is recorded.
    pub egress: Arc<EgressLog>,
    /// The time each request started.
    pub clock: Arc<dyn Clock>,
}

/// The network, as the composition root holds it: it only hands out
/// [`ScopedClient`]s.
#[derive(Clone)]
pub struct Net {
    parts: Arc<NetParts>,
}

impl Net {
    /// Wraps a root client with policy and the egress log.
    pub fn new(parts: NetParts) -> Self {
        Self {
            parts: Arc::new(parts),
        }
    }

    /// A client that can only reach `service`'s hosts, while it is enabled
    /// and offline mode is off.
    pub fn scoped(&self, service: ServiceId) -> ScopedClient {
        self.client(service, Purpose::Regular)
    }

    /// A client for model downloads the user explicitly started: like
    /// [`Net::scoped`], but it still works in offline mode. Fails for a
    /// service that does not download model files.
    pub fn approved_download(&self, service: ServiceId) -> Result<ScopedClient, NetError> {
        if !registry::spec(service).offline_downloads {
            return Err(NetError::NotADownloadService(service));
        }
        Ok(self.client(service, Purpose::ApprovedDownload))
    }

    /// The egress log every scoped client writes to.
    pub fn egress(&self) -> &Arc<EgressLog> {
        &self.parts.egress
    }

    fn client(&self, service: ServiceId, purpose: Purpose) -> ScopedClient {
        ScopedClient {
            service,
            purpose,
            parts: Arc::clone(&self.parts),
        }
    }
}

/// Why a scoped client exists, which decides whether offline mode stops it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Purpose {
    /// Ordinary traffic: stopped by offline mode.
    Regular,
    /// A download the user started: allowed in offline mode.
    ApprovedDownload,
}

/// A [`NetClient`] confined to one service (see the module docs).
#[derive(Clone)]
pub struct ScopedClient {
    service: ServiceId,
    purpose: Purpose,
    parts: Arc<NetParts>,
}

impl ScopedClient {
    /// The service this client is confined to.
    pub fn service(&self) -> ServiceId {
        self.service
    }

    /// Fails unless policy lets `request` leave the Mac now.
    async fn authorize(&self, request: &Request) -> Result<(), NetError> {
        let grant = self.parts.policy.grant(self.service).await;
        let host = request.url.host_str().unwrap_or_default();
        let spec = registry::spec(self.service);
        let custom = grant.custom_hosts.iter().filter(|_| spec.custom_hosts);
        if !spec
            .hosts
            .iter()
            .chain(custom)
            .any(|allowed| allowed.matches(host))
        {
            return Err(NetError::HostNotAllowed {
                service: self.service,
                host: host.to_owned(),
            });
        }
        if !grant.enabled {
            return Err(NetError::ServiceDisabled(self.service));
        }
        if grant.offline_mode && self.purpose != Purpose::ApprovedDownload {
            return Err(NetError::Offline(self.service));
        }
        Ok(())
    }

    /// Sends one request without following redirects, and records it.
    async fn hop(&self, request: Request) -> Result<Response, NetError> {
        let exchange = Exchange::new(Arc::clone(&self.parts.egress), self.entry(&request));
        if let Err(error) = self.authorize(&request).await {
            exchange.finish(EgressOutcome::Blocked);
            return Err(error);
        }
        match self.parts.client.send(request).await {
            Err(error) => {
                exchange.finish(EgressOutcome::Failed);
                Err(error)
            }
            Ok(response) => {
                let body = exchange.meter(response.status.as_u16(), response.body);
                Ok(Response {
                    body: body.boxed(),
                    ..response
                })
            }
        }
    }

    /// The egress entry for `request`, before its outcome is known.
    fn entry(&self, request: &Request) -> EgressEntry {
        EgressEntry {
            at: self.parts.clock.now(),
            service: self.service,
            host: request.authority(),
            method: request.method,
            status: None,
            outcome: EgressOutcome::Sent,
            bytes_up: request.body_len(),
            bytes_down: 0,
        }
    }
}

#[async_trait]
impl NetClient for ScopedClient {
    /// Sends `request`, following up to [`MAX_REDIRECTS`] redirects, each
    /// one checked and recorded like the first request.
    async fn send(&self, mut request: Request) -> Result<Response, NetError> {
        for _ in 0..=MAX_REDIRECTS {
            let response = self.hop(request.clone()).await?;
            match redirect::next(&request, &response)? {
                Some(next) => request = next,
                None => return Ok(response),
            }
        }
        Err(NetError::Redirect)
    }
}

#[cfg(test)]
mod scoped_tests;
