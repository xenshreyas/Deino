//! Test helpers behind `test-support`: a local OAuth authorization server
//! and a browser that only speaks enough HTTP to follow a redirect. Nothing
//! here touches the network beyond `127.0.0.1`.

mod auth_server;
mod http;

pub use auth_server::{MockAuthorizationServer, MockServerOptions};
pub use http::browser_get;
