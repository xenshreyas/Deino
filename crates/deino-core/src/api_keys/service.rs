//! [`ApiKeyService`]: makes, lists, revokes and checks API keys.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{ApiKeyId, Clock};

use super::{
    ApiCaller, ApiKey, ApiKeyError, ApiKeyRepo, CreatedApiKey, KeyToken, MAX_KEY_NAME_CHARS,
    NewApiKey, SecretHasher, StoredApiKey, TokenSource,
};

/// How stale `last_used_at` may get before a request writes it again.
const TOUCH_EVERY: TimeDelta = TimeDelta::minutes(1);

/// What the service works through.
pub struct ApiKeyDeps {
    /// The keys.
    pub repo: Arc<dyn ApiKeyRepo>,
    /// Hashes and checks secrets.
    pub hasher: Arc<dyn SecretHasher>,
    /// Makes new secrets.
    pub tokens: Arc<dyn TokenSource>,
    /// Stamps creation, revocation and use.
    pub clock: Arc<dyn Clock>,
}

/// The rules for API keys: the token is shown once, only its hash is kept,
/// and a revoked or unknown key is refused like a wrong one.
pub struct ApiKeyService {
    deps: ApiKeyDeps,
}

impl ApiKeyService {
    /// The service over its ports.
    pub fn new(deps: ApiKeyDeps) -> Self {
        Self { deps }
    }

    /// Every key, newest first, revoked ones included.
    pub async fn list(&self) -> Result<Vec<ApiKey>, ApiKeyError> {
        Ok(self.deps.repo.list().await?)
    }

    /// Makes a key and returns it with its token, which is never shown again.
    pub async fn create(&self, new: &NewApiKey) -> Result<CreatedApiKey, ApiKeyError> {
        let name = valid_name(&new.name)?;
        let secret = self.deps.tokens.secret().map_err(ApiKeyError::Crypto)?;
        let hasher = Arc::clone(&self.deps.hasher);
        let to_hash = secret.clone();
        let secret_hash = blocking(move || hasher.hash(&to_hash))
            .await?
            .map_err(ApiKeyError::Crypto)?;
        let key = ApiKey {
            id: ApiKeyId::generate(self.deps.clock.as_ref()),
            name,
            scope: new.scope,
            created_at: self.deps.clock.now(),
            last_used_at: None,
            revoked_at: None,
        };
        let stored = StoredApiKey { key, secret_hash };
        self.deps.repo.insert(&stored).await?;
        log::info!(target: "api", "created API key {} ({:?})", stored.key.id, stored.key.scope);
        let token = KeyToken {
            id: stored.key.id,
            secret,
        };
        Ok(CreatedApiKey {
            token: token.render(),
            key: stored.key,
        })
    }

    /// Revokes the key at once; requests with it are refused from now on.
    pub async fn revoke(&self, id: &ApiKeyId) -> Result<ApiKey, ApiKeyError> {
        self.find(id).await?;
        self.deps.repo.revoke(id, self.deps.clock.now()).await?;
        log::info!(target: "api", "revoked API key {id}");
        Ok(self.find(id).await?.key)
    }

    /// Removes the key from the list (revoking it, if it was not yet).
    pub async fn delete(&self, id: &ApiKeyId) -> Result<(), ApiKeyError> {
        if self.deps.repo.delete(id).await? {
            Ok(())
        } else {
            Err(ApiKeyError::NotFound(*id))
        }
    }

    /// The caller `token` names, when it is a live key's; otherwise
    /// [`ApiKeyError::Unauthorized`]. Records the use, at most once a minute.
    pub async fn authenticate(&self, token: &str) -> Result<ApiCaller, ApiKeyError> {
        let parsed = KeyToken::parse(token).ok_or(ApiKeyError::Unauthorized)?;
        let stored = self
            .deps
            .repo
            .find(&parsed.id)
            .await?
            .filter(|stored| stored.key.revoked_at.is_none())
            .ok_or(ApiKeyError::Unauthorized)?;
        let hasher = Arc::clone(&self.deps.hasher);
        let hash = stored.secret_hash.clone();
        if !blocking(move || hasher.verify(&parsed.secret, &hash)).await? {
            return Err(ApiKeyError::Unauthorized);
        }
        self.touch(&stored.key).await?;
        Ok(ApiCaller {
            key_id: stored.key.id,
            scope: stored.key.scope,
        })
    }

    async fn touch(&self, key: &ApiKey) -> Result<(), ApiKeyError> {
        let now = self.deps.clock.now();
        let stale = key.last_used_at.is_none_or(|used| now - used >= TOUCH_EVERY);
        if stale {
            self.deps.repo.touch(&key.id, now).await?;
        }
        Ok(())
    }

    async fn find(&self, id: &ApiKeyId) -> Result<StoredApiKey, ApiKeyError> {
        self.deps
            .repo
            .find(id)
            .await?
            .ok_or(ApiKeyError::NotFound(*id))
    }
}

/// The trimmed name, when it is 1 to [`MAX_KEY_NAME_CHARS`] characters.
fn valid_name(name: &str) -> Result<String, ApiKeyError> {
    let name = name.trim();
    let chars = name.chars().count();
    if chars == 0 || chars > MAX_KEY_NAME_CHARS {
        return Err(ApiKeyError::InvalidName);
    }
    Ok(name.to_owned())
}

/// Runs argon2 off the async workers: it takes milliseconds of CPU.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ApiKeyError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| ApiKeyError::Crypto(error.to_string()))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
