//! API keys for the local REST API (DEV-03, ADR 2026-10-12-local-rest-api).
//!
//! A key is read-only or read-write. Its token, `deino_<id>_<secret>`, is
//! shown once when the key is made (Settings → Developer); the `api_keys`
//! table keeps only an argon2id hash of the secret. Keys are listed with
//! when they were last used, and revoked at once.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`ApiKey`], [`ApiKeyScope`], [`NewApiKey`], [`CreatedApiKey`], [`ApiCaller`], [`KeyToken`] |
//! | `hasher.rs` | [`SecretHasher`] ([`Argon2Hasher`]) and [`TokenSource`] ([`OsTokens`]) |
//! | `repo.rs` | [`ApiKeyRepo`], [`SqliteApiKeyRepo`] |
//! | `service.rs` | [`ApiKeyService`]: create, list, revoke, delete, authenticate |

mod error;
mod hasher;
mod model;
mod repo;
mod service;

pub use error::ApiKeyError;
pub use hasher::{Argon2Hasher, OsTokens, SECRET_BYTES, SecretHasher, TokenSource};
pub use model::{
    ApiCaller, ApiKey, ApiKeyScope, CreatedApiKey, KeyToken, MAX_KEY_NAME_CHARS, NewApiKey,
    StoredApiKey, TOKEN_PREFIX,
};
pub use repo::{ApiKeyRepo, SqliteApiKeyRepo};
pub use service::{ApiKeyDeps, ApiKeyService};
