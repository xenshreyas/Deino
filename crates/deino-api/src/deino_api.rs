//! The local REST API (DEV-03, ADR 2026-10-12-local-rest-api).
//!
//! An `axum` server on `127.0.0.1` only, off by default, on the port in
//! `settings.api`. Every request needs a key from Settings → Developer
//! (`Authorization: Bearer deino_…`); read-only keys may only `GET`. While
//! Deino is locked (PRV-02) every request gets `423 Locked`.
//!
//! Reads go through `deino-core::query`, the `deino` CLI's read layer, so
//! the API, `deino --json` and the MCP tools return the same objects.
//! Writes go through the app's own services, so every change takes the
//! single writer and the UI updates through `DbChanged`. Lists page with
//! opaque cursors and an RFC 8288 `Link` header; failures are
//! `{"error": {"code", "message"}}`.
//!
//! | File | Role |
//! |---|---|
//! | `server.rs` | [`RunningServer`], [`bind_address`] (loopback only) |
//! | `controller.rs` | [`ApiController`]: start and stop with the setting; [`ApiStatus`] |
//! | `routes.rs`, `routes/` | [`router`] and the `/v1` handlers |
//! | `guard.rs` | host, key, scope and lock checks; request logs without bodies |
//! | `page.rs` | cursors, [`Page`], the `Link` header |
//! | `error.rs` | [`ApiError`], mapped from `AppError` |
//! | `state.rs` | [`ApiServices`], [`WriteServices`], [`LockProbe`] |
//!
//! No HTTP client lives here: the server only answers (ADR 0019).

mod controller;
mod error;
mod extract;
mod guard;
mod page;
mod routes;
mod server;
mod state;

pub use controller::{ApiController, ApiStatus};
pub use error::{ApiError, ApiResult};
pub use page::{DEFAULT_PAGE, MAX_PAGE, Page};
pub use routes::router;
pub use server::{RunningServer, ServeError, bind_address};
pub use state::{ApiServices, ApiState, LockProbe, NeverLocked, WriteServices};
