//! [`NetworkControls`]: what Settings → Privacy → Network shows and changes.
//!
//! It lists every service with its hosts and whether it may send, switches a
//! service on or off, and reads the egress log. It sits on the same ports the
//! scoped clients read, so the page always shows what is actually enforced.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, ServiceId, ServiceKind};
use serde::Serialize;

use crate::{
    EgressEntry, EgressReader, HostPattern, ServicePolicy, ServiceSwitch, SwitchError, registry,
};

/// The most egress log entries one call returns.
pub const MAX_EGRESS_PAGE: u32 = 500;

/// A service as the Network page lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    /// The service.
    pub id: ServiceId,
    /// What kind of service it is.
    pub kind: ServiceKind,
    /// The hosts it may reach, custom endpoints included (`*.hf.co` for a
    /// family of subdomains).
    pub hosts: Vec<String>,
    /// Whether it may send now (offline mode aside).
    pub enabled: bool,
}

/// A Network page failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ControlsError {
    /// The switch could not be saved.
    #[error(transparent)]
    Switch(#[from] SwitchError),
    /// The egress log could not be read.
    #[error("could not read the network log")]
    Log(#[from] DbError),
}

impl ErrorClass for ControlsError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Switch(_) => ErrorCode::Internal,
            Self::Log(error) => error.code(),
        }
    }
}

/// Lists and switches services, and reads the egress log.
pub struct NetworkControls {
    policy: Arc<dyn ServicePolicy>,
    switch: Arc<dyn ServiceSwitch>,
    egress: Arc<dyn EgressReader>,
}

impl NetworkControls {
    /// Controls over the user's choices and an egress log reader.
    pub fn new(
        policy: Arc<dyn ServicePolicy>,
        switch: Arc<dyn ServiceSwitch>,
        egress: Arc<dyn EgressReader>,
    ) -> Self {
        Self {
            policy,
            switch,
            egress,
        }
    }

    /// Every service, in registry order.
    pub async fn services(&self) -> Vec<ServiceStatus> {
        let mut services = Vec::with_capacity(ServiceId::ALL.len());
        for &service in ServiceId::ALL {
            services.push(self.status(service).await);
        }
        services
    }

    /// Switches `service` on or off and returns its new status.
    pub async fn set_service_enabled(
        &self,
        service: ServiceId,
        enabled: bool,
    ) -> Result<ServiceStatus, ControlsError> {
        self.switch.set_enabled(service, enabled).await?;
        Ok(self.status(service).await)
    }

    /// The newest `limit` egress log entries (at most [`MAX_EGRESS_PAGE`]),
    /// newest first.
    pub async fn egress_log(&self, limit: u32) -> Result<Vec<EgressEntry>, ControlsError> {
        Ok(self.egress.recent(limit.min(MAX_EGRESS_PAGE)).await?)
    }

    async fn status(&self, service: ServiceId) -> ServiceStatus {
        let grant = self.policy.grant(service).await;
        let spec = registry::spec(service);
        let custom = grant.custom_hosts.iter().filter(|_| spec.custom_hosts);
        ServiceStatus {
            id: service,
            kind: service.kind(),
            hosts: spec.hosts.iter().chain(custom).map(HostPattern::display).collect(),
            enabled: grant.enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EgressLog, FakeServicePolicy};

    fn controls() -> (NetworkControls, Arc<FakeServicePolicy>) {
        let policy = Arc::new(FakeServicePolicy::default());
        let controls = NetworkControls::new(
            policy.clone(),
            policy.clone(),
            Arc::new(EgressLog::default()),
        );
        (controls, policy)
    }

    #[tokio::test]
    async fn every_service_is_listed_with_its_hosts_and_state() {
        let (controls, policy) = controls();
        policy.add_custom_host(ServiceId::LlmOllama, "gpu-box.local");

        let services = controls.services().await;

        assert_eq!(services.len(), ServiceId::ALL.len());
        let ollama = services
            .iter()
            .find(|status| status.id == ServiceId::LlmOllama)
            .unwrap();
        assert!(ollama.enabled);
        assert!(ollama.hosts.contains(&"gpu-box.local".to_owned()));
        let hf = services.iter().find(|s| s.id == ServiceId::HfModels).unwrap();
        assert!(hf.hosts.contains(&"*.hf.co".to_owned()));
    }

    #[tokio::test]
    async fn switching_a_service_returns_its_new_state() {
        let (controls, _) = controls();

        let notion = controls
            .set_service_enabled(ServiceId::Notion, true)
            .await
            .unwrap();

        assert!(notion.enabled);
        assert_eq!(notion.kind, ServiceKind::Integration);
    }
}
