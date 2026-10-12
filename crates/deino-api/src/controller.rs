//! [`ApiController`]: keeps the server in step with `settings.api`. It
//! starts the server when the setting is on, stops it when it goes off,
//! and restarts it on a new port.

use std::sync::Arc;

use deino_core::settings::{ApiSettings, SettingsError, SettingsService};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{ApiServices, RunningServer};

/// The server as Settings → Developer shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiStatus {
    /// The setting: serve the API.
    pub enabled: bool,
    /// The setting: the port.
    pub port: u16,
    /// The base URL while it listens, such as `http://127.0.0.1:47810/v1`.
    pub base_url: Option<String>,
    /// Why it is not listening although enabled (the port is taken, say).
    pub error: Option<String>,
}

#[derive(Default)]
struct Server {
    running: Option<RunningServer>,
    error: Option<String>,
}

/// Starts and stops the API with its setting.
pub struct ApiController {
    services: Arc<ApiServices>,
    settings: Arc<SettingsService>,
    server: Mutex<Server>,
}

impl ApiController {
    /// A controller over the API's services and the settings.
    pub fn new(services: Arc<ApiServices>, settings: Arc<SettingsService>) -> Self {
        Self {
            services,
            settings,
            server: Mutex::new(Server::default()),
        }
    }

    /// The setting and whether the server listens.
    pub async fn status(&self) -> ApiStatus {
        let wanted = self.settings.get().await.api;
        status(wanted, &*self.server.lock().await)
    }

    /// Saves `wanted` as the setting and brings the server in line.
    pub async fn configure(&self, wanted: ApiSettings) -> Result<ApiStatus, SettingsError> {
        self.settings.update(|settings| settings.api = wanted).await?;
        Ok(self.reconcile().await)
    }

    /// Starts, stops or moves the server to match the saved setting.
    pub async fn reconcile(&self) -> ApiStatus {
        let wanted = self.settings.get().await.api;
        let mut server = self.server.lock().await;
        let keep = server
            .running
            .as_ref()
            .is_some_and(|running| wanted.enabled && running.address().port() == wanted.port);
        if !keep {
            if let Some(running) = server.running.take() {
                running.stop().await;
            }
            *server = if wanted.enabled {
                self.start(wanted.port).await
            } else {
                Server::default()
            };
        }
        status(wanted, &server)
    }

    async fn start(&self, port: u16) -> Server {
        match RunningServer::start(Arc::clone(&self.services), port).await {
            Ok(running) => Server {
                running: Some(running),
                error: None,
            },
            Err(error) => {
                log::warn!(target: "api", "{error}");
                Server {
                    running: None,
                    error: Some(error.to_string()),
                }
            }
        }
    }

    /// Stops the server, at exit.
    pub async fn shutdown(&self) {
        if let Some(running) = self.server.lock().await.running.take() {
            running.stop().await;
        }
    }
}

fn status(wanted: ApiSettings, server: &Server) -> ApiStatus {
    ApiStatus {
        enabled: wanted.enabled,
        port: wanted.port,
        base_url: server
            .running
            .as_ref()
            .map(|running| format!("http://{}/v1", running.address())),
        error: server.error.clone(),
    }
}
