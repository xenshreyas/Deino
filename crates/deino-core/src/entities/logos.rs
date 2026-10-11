//! Company logos: the favicon at `https://<domain>/favicon.ico`, fetched
//! once per company and cached on the Mac.
//!
//! [`LogoService`] owns the rules; [`LogoSource`] (the network,
//! [`FaviconSource`](super::FaviconSource) over the `favicons` service) and
//! [`LogoFiles`] (the cache folder, [`FsLogoFiles`](super::FsLogoFiles)) are
//! its ports. The `favicons` service is off until the user switches it on
//! (Settings → Network), and offline mode stops it like any other: a fetch
//! the network policy refuses is not recorded as tried, so logos appear
//! once the user allows them. Any other outcome (a logo, no logo, an
//! unreachable site) is recorded in `companies.logo_checked_at`, and the
//! site is not asked again unless the user asks for a refresh.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Clock, CompanyId, ErrorClass, ErrorCode};
use deino_net::NetError;

use super::{EntityDirectory, EntityError, EntityWrites};
use crate::companies::{CompanyDomain, CompanyRepo};

/// The image formats a logo may be. SVG is refused: it can carry scripts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    /// A Windows icon, what most `favicon.ico` files are.
    Ico,
    /// PNG.
    Png,
    /// GIF.
    Gif,
    /// JPEG.
    Jpeg,
    /// WebP.
    Webp,
}

impl ImageKind {
    /// Every kind, for cleaning up a company's older logo files.
    pub const ALL: [Self; 5] = [Self::Ico, Self::Png, Self::Gif, Self::Jpeg, Self::Webp];

    /// The kind `bytes` start like, by their magic number.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0, 0, 1, 0, ..] => Some(Self::Ico),
            [0x89, b'P', b'N', b'G', ..] => Some(Self::Png),
            [b'G', b'I', b'F', b'8', ..] => Some(Self::Gif),
            [0xFF, 0xD8, 0xFF, ..] => Some(Self::Jpeg),
            [
                b'R',
                b'I',
                b'F',
                b'F',
                _,
                _,
                _,
                _,
                b'W',
                b'E',
                b'B',
                b'P',
                ..,
            ] => Some(Self::Webp),
            _ => None,
        }
    }

    /// The file extension.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Ico => "ico",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }
}

/// A downloaded logo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logo {
    /// The image file's bytes.
    pub bytes: Vec<u8>,
    /// What they are.
    pub kind: ImageKind,
}

/// A logo that could not be fetched or stored.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LogoError {
    /// The request failed or was refused; the source says why.
    #[error(transparent)]
    Network(#[from] NetError),
    /// The cache folder could not be written.
    #[error("could not save the logo")]
    File(#[from] std::io::Error),
}

impl ErrorClass for LogoError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Network(error) => error.code(),
            Self::File(_) => ErrorCode::Internal,
        }
    }
}

impl LogoError {
    /// True when the network policy refused the request (the service is
    /// off, offline mode is on): nothing was asked, so try again later.
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Network(error) if error.code() == ErrorCode::PermissionDenied)
    }
}

/// Where logos come from.
#[async_trait]
pub trait LogoSource: Send + Sync {
    /// The favicon of `domain`; `None` when the site has none, or none
    /// that is a usable image.
    async fn fetch(&self, domain: &CompanyDomain) -> Result<Option<Logo>, LogoError>;
}

/// Where logos are cached.
#[async_trait]
pub trait LogoFiles: Send + Sync {
    /// Stores `logo` as the company's, replacing any older one, and returns
    /// the file's path.
    async fn save(&self, company: &CompanyId, logo: &Logo) -> Result<String, LogoError>;
}

/// What [`LogoService`] is built from.
#[derive(Clone)]
pub struct LogoPorts {
    /// Companies, for their domain and fetch record.
    pub companies: Arc<dyn CompanyRepo>,
    /// Merged companies resolve to their survivor.
    pub directory: Arc<EntityDirectory>,
    /// Records the outcome.
    pub writes: Arc<dyn EntityWrites>,
    /// The network.
    pub source: Arc<dyn LogoSource>,
    /// The cache folder.
    pub files: Arc<dyn LogoFiles>,
    /// When each fetch happened.
    pub clock: Arc<dyn Clock>,
}

/// Fetches and caches company logos (see the module docs).
pub struct LogoService {
    ports: LogoPorts,
}

impl LogoService {
    /// Creates the service over its ports.
    pub fn new(ports: LogoPorts) -> Self {
        Self { ports }
    }

    /// The company's logo path, fetched first unless the site was asked
    /// before (or always, with `refresh`). `None` for a company without a
    /// domain or whose site has no usable favicon.
    pub async fn ensure(
        &self,
        id: &CompanyId,
        refresh: bool,
    ) -> Result<Option<String>, EntityError> {
        let company = self.ports.companies.find(id).await?;
        let company = company.ok_or(EntityError::CompanyNotFound(*id))?;
        let company = self.ports.directory.surviving_company(company).await?;
        let Some(domain) = company.domain.as_ref() else {
            return Ok(company.logo_path);
        };
        if company.logo_checked_at.is_some() && !refresh {
            return Ok(company.logo_path);
        }
        let saved = match self.fetch(&company.id, domain).await {
            Err(error) if error.is_refused() => return Err(error.into()),
            Err(LogoError::Network(_)) => None,
            other => other?,
        };
        let now = self.ports.clock.now();
        self.ports
            .writes
            .set_company_logo(&company.id, saved.as_deref(), now)
            .await?;
        Ok(saved.or(company.logo_path))
    }

    /// Downloads and stores the favicon of `domain` for `company`.
    async fn fetch(
        &self,
        company: &CompanyId,
        domain: &CompanyDomain,
    ) -> Result<Option<String>, LogoError> {
        match self.ports.source.fetch(domain).await? {
            Some(logo) => self.ports.files.save(company, &logo).await.map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod logos_tests;
