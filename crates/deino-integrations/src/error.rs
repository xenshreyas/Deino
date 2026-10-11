//! What can go wrong: [`ProviderError`] inside an adapter, and
//! [`IntegrationError`], what the UI sees.
//!
//! Every message is user-safe: it may name a provider, a status or a limit,
//! never a token, a URL with a query, or anything the user wrote.

use std::time::Duration;

use deino_db::DbError;
use deino_domain::{Capability, ConnectionId, ErrorClass, ErrorCode, IntegrationProvider};
use deino_net::NetError;
use deino_secrets::SecretError;

use crate::{Failure, OAuthError};

/// A failure inside a provider adapter, before the service decides what it
/// means for the connection.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProviderError {
    /// A non-2xx answer the provider's classifier has not looked at yet.
    #[error("the provider answered {}", .0.status)]
    Http(Failure),
    /// The provider no longer accepts the credential.
    #[error("the provider no longer accepts the saved sign-in")]
    Unauthorized,
    /// The provider asks Deino to slow down, for `retry_after` if it said.
    #[error("the provider is limiting requests")]
    RateLimited {
        /// How long the provider asked to wait.
        retry_after: Option<Duration>,
    },
    /// The provider is down or overloaded.
    #[error("the provider is unavailable right now")]
    Unavailable,
    /// The provider refused the request; `reason` is user-safe.
    #[error("{reason}")]
    Rejected {
        /// Why, in words the user can act on.
        reason: String,
    },
    /// The provider answered with something the adapter cannot read.
    #[error("the provider's answer could not be read")]
    Unexpected,
    /// The request never got an answer (policy, transport, decoding).
    #[error(transparent)]
    Net(#[from] NetError),
}

impl ProviderError {
    /// True when sending again later may succeed.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::Unavailable => true,
            Self::Net(error) => is_transient(error),
            _ => false,
        }
    }
}

/// True for transport failures that may pass (not policy, not a bad answer).
fn is_transient(error: &NetError) -> bool {
    matches!(
        error,
        NetError::Connect(_) | NetError::Timeout(_) | NetError::Transport(_)
    )
}

/// An integrations failure, as the UI sees it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IntegrationError {
    /// No provider with this ID is available in this build.
    #[error("{} is not available in this version of Deino", .0.name())]
    UnknownProvider(IntegrationProvider),
    /// The connection does not exist (any more).
    #[error("connection {0} was not found")]
    ConnectionNotFound(ConnectionId),
    /// The provider does not offer this way to connect.
    #[error("{} cannot be connected that way", .0.name())]
    MethodUnavailable(IntegrationProvider),
    /// The provider cannot do what was asked.
    #[error("{} cannot {}", .provider.name(), .capability.as_str().replace('_', " "))]
    Unsupported {
        /// The provider.
        provider: IntegrationProvider,
        /// What it was asked to do.
        capability: Capability,
    },
    /// OAuth needs the user's own app (client ID, maybe a secret) first.
    #[error("add your {} app's client ID first", .0.name())]
    AppNotConfigured(IntegrationProvider),
    /// Offline mode is on.
    #[error("offline mode is on, so {} cannot be reached", .0.name())]
    Offline(IntegrationProvider),
    /// The user switched the provider's service off.
    #[error("{} is switched off in Settings → Privacy → Network", .0.name())]
    Disabled(IntegrationProvider),
    /// Another connect to this provider is still waiting for the browser.
    #[error("{} is already waiting for you in the browser", .0.name())]
    AlreadyConnecting(IntegrationProvider),
    /// The user cancelled, or the browser never came back.
    #[error("connecting to {} was cancelled", .0.name())]
    Cancelled(IntegrationProvider),
    /// The sign-in did not complete.
    #[error("{} sign-in failed: {source}", .provider.name())]
    OAuth {
        /// The provider.
        provider: IntegrationProvider,
        /// What went wrong, user-safe.
        #[source]
        source: OAuthError,
    },
    /// The provider no longer accepts the credential and it cannot be
    /// refreshed.
    #[error("{} no longer accepts the saved sign-in; connect again", .0.name())]
    NeedsReconnect(IntegrationProvider),
    /// The provider is limiting requests.
    #[error("{} is limiting requests; try again in a minute", .0.name())]
    RateLimited(IntegrationProvider),
    /// The provider is down or overloaded.
    #[error("{} is unavailable right now", .0.name())]
    Unavailable(IntegrationProvider),
    /// The provider refused the request.
    #[error("{}: {reason}", .provider.name())]
    Rejected {
        /// The provider.
        provider: IntegrationProvider,
        /// Why, user-safe.
        reason: String,
    },
    /// The provider could not be reached, or answered something unreadable.
    #[error("could not reach {}", .provider.name())]
    Network {
        /// The provider.
        provider: IntegrationProvider,
        /// The transport failure, for the log.
        #[source]
        source: Option<NetError>,
    },
    /// A delivery with this idempotency key is being sent right now.
    #[error("this is already being sent")]
    InFlight,
    /// Input the user can fix: a blank token, a bad client ID.
    #[error("{0}")]
    Invalid(String),
    /// The Keychain refused or failed.
    #[error(transparent)]
    Secrets(#[from] SecretError),
    /// The database failed.
    #[error("could not load or save integrations")]
    Storage(#[from] DbError),
    /// A setting could not be saved.
    #[error("could not save the integration settings")]
    Settings,
}

impl ErrorClass for IntegrationError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::UnknownProvider(_) | Self::ConnectionNotFound(_) => ErrorCode::NotFound,
            Self::MethodUnavailable(_)
            | Self::Unsupported { .. }
            | Self::AppNotConfigured(_)
            | Self::Rejected { .. }
            | Self::Invalid(_) => ErrorCode::Invalid,
            Self::Offline(_) => ErrorCode::Unavailable,
            Self::Disabled(_) | Self::NeedsReconnect(_) => ErrorCode::PermissionDenied,
            Self::AlreadyConnecting(_) | Self::InFlight => ErrorCode::Conflict,
            Self::Cancelled(_) => ErrorCode::Cancelled,
            Self::OAuth { source, .. } => source.code(),
            Self::RateLimited(_) | Self::Unavailable(_) | Self::Network { .. } => {
                ErrorCode::Network
            }
            Self::Secrets(error) => error.code(),
            Self::Storage(_) | Self::Settings => ErrorCode::Internal,
        }
    }
}

impl IntegrationError {
    /// True when sending the same delivery again later may succeed: the
    /// provider was limiting requests, down, or unreachable. An automation
    /// stage maps this to the pipeline's retryable stage error, so the FND-08
    /// runner retries it with backoff (and the idempotency key keeps that
    /// from writing twice).
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited(_) | Self::Unavailable(_) | Self::Network { .. } | Self::InFlight
        )
    }

    /// What `error` from `provider`'s adapter means for the user. An
    /// unclassified [`ProviderError::Http`] counts as a refusal.
    pub fn from_provider(provider: IntegrationProvider, error: ProviderError) -> Self {
        match error {
            ProviderError::Unauthorized => Self::NeedsReconnect(provider),
            ProviderError::RateLimited { .. } => Self::RateLimited(provider),
            ProviderError::Unavailable => Self::Unavailable(provider),
            ProviderError::Rejected { reason } => Self::Rejected { provider, reason },
            ProviderError::Http(failure) => Self::Rejected {
                provider,
                reason: format!("the request was refused ({})", failure.status),
            },
            ProviderError::Unexpected => Self::Network {
                provider,
                source: None,
            },
            ProviderError::Net(error) => Self::from_net(provider, error),
        }
    }

    /// What a request `error` means: policy refusals keep their reason.
    pub fn from_net(provider: IntegrationProvider, error: NetError) -> Self {
        match error {
            NetError::Offline(_) => Self::Offline(provider),
            NetError::ServiceDisabled(_) => Self::Disabled(provider),
            error => Self::Network {
                provider,
                source: Some(error),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{AppError, ServiceId};

    use super::*;

    #[test]
    fn offline_mode_and_switched_off_services_say_why_nothing_was_sent() {
        let offline = IntegrationError::from_net(
            IntegrationProvider::Slack,
            NetError::Offline(ServiceId::Slack),
        );
        let off = IntegrationError::from_net(
            IntegrationProvider::Notion,
            NetError::ServiceDisabled(ServiceId::Notion),
        );

        assert_eq!(
            AppError::from(offline),
            AppError::new(
                ErrorCode::Unavailable,
                "offline mode is on, so Slack cannot be reached"
            )
        );
        assert_eq!(
            AppError::from(off).message,
            "Notion is switched off in Settings → Privacy → Network"
        );
    }

    #[test]
    fn an_unauthorized_answer_asks_the_user_to_connect_again() {
        let error = IntegrationError::from_provider(
            IntegrationProvider::Linear,
            ProviderError::Unauthorized,
        );

        let app = AppError::from(error);

        assert_eq!(app.code, ErrorCode::PermissionDenied);
        assert_eq!(
            app.message,
            "Linear no longer accepts the saved sign-in; connect again"
        );
    }

    #[test]
    fn only_rate_limits_outages_and_transport_failures_are_retryable() {
        assert!(ProviderError::Unavailable.is_retryable());
        assert!(ProviderError::RateLimited { retry_after: None }.is_retryable());
        assert!(!ProviderError::Unauthorized.is_retryable());
        assert!(!ProviderError::Net(NetError::Offline(ServiceId::Slack)).is_retryable());
        let unsupported = IntegrationError::Unsupported {
            provider: IntegrationProvider::Slack,
            capability: Capability::CreateTask,
        };
        assert_eq!(unsupported.to_string(), "Slack cannot create task");
    }
}
