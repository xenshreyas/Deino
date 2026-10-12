//! [`FaviconSource`]: the [`LogoSource`] over the network, and
//! [`FsLogoFiles`]: the [`LogoFiles`] cache folder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use deino_domain::CompanyId;
use deino_net::{NetClient, NetError, Request};

use super::{ImageKind, Logo, LogoError, LogoFiles, LogoSource};
use crate::companies::CompanyDomain;

/// The largest favicon accepted; real ones are a few kilobytes.
pub const MAX_LOGO_BYTES: usize = 512 * 1024;

/// How long a site may take to answer.
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Fetches `https://<domain>/favicon.ico` through a client scoped to the
/// `favicons` service, so the user's switch and offline mode apply.
pub struct FaviconSource {
    client: Arc<dyn NetClient>,
}

impl FaviconSource {
    /// A source over `client` (`net.scoped(ServiceId::Favicons)` in the app).
    pub fn new(client: Arc<dyn NetClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LogoSource for FaviconSource {
    async fn fetch(&self, domain: &CompanyDomain) -> Result<Option<Logo>, LogoError> {
        let url = format!("https://{}/favicon.ico", domain.as_str());
        let request = Request::get(&url)?
            .header("accept", "image/*")
            .timeout(Some(FETCH_TIMEOUT));
        let response = self.client.send(request).await?;
        if !response.status.is_success() {
            return Ok(None);
        }
        let bytes = match response.bytes(MAX_LOGO_BYTES).await {
            Err(NetError::TooLarge { .. }) => return Ok(None),
            other => other?,
        };
        Ok(ImageKind::sniff(&bytes).map(|kind| Logo {
            bytes: bytes.to_vec(),
            kind,
        }))
    }
}

/// Keeps one logo file per company, `<company id>.<extension>`, in a folder
/// under the app's data folder.
pub struct FsLogoFiles {
    folder: PathBuf,
}

impl FsLogoFiles {
    /// A cache in `folder`, created on the first save.
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }
}

#[async_trait]
impl LogoFiles for FsLogoFiles {
    async fn save(&self, company: &CompanyId, logo: &Logo) -> Result<String, LogoError> {
        tokio::fs::create_dir_all(&self.folder).await?;
        let path = self
            .folder
            .join(format!("{company}.{}", logo.kind.extension()));
        // Written aside and renamed, so a reader never sees half a file.
        let partial = path.with_extension("partial");
        tokio::fs::write(&partial, &logo.bytes).await?;
        tokio::fs::rename(&partial, &path).await?;
        for kind in ImageKind::ALL.into_iter().filter(|kind| *kind != logo.kind) {
            let older = self.folder.join(format!("{company}.{}", kind.extension()));
            // Most kinds were never stored; a missing file is the usual case.
            if let Err(error) = tokio::fs::remove_file(older).await
                && error.kind() != std::io::ErrorKind::NotFound
            {
                log::warn!(target: "entities", "could not remove an older logo of {company}: {error}");
            }
        }
        Ok(path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_net::{FakeNetClient, FakeReply, Status};
    use deino_testkit::at;

    use super::*;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

    fn domain() -> CompanyDomain {
        CompanyDomain::parse("brightkite-logistics.com").unwrap()
    }

    #[tokio::test]
    async fn a_favicon_is_fetched_from_the_sites_root_and_sniffed() {
        let fake = Arc::new(FakeNetClient::default());
        fake.reply(FakeReply::ok(PNG));
        let source = FaviconSource::new(fake.clone());

        let logo = source.fetch(&domain()).await.unwrap().unwrap();

        assert_eq!(logo.kind, ImageKind::Png);
        let requests = fake.requests();
        assert_eq!(
            requests[0].url.as_str(),
            "https://brightkite-logistics.com/favicon.ico"
        );
    }

    #[tokio::test]
    async fn missing_oversized_and_non_image_favicons_are_no_logo() {
        let fake = Arc::new(FakeNetClient::default());
        fake.reply(FakeReply::status(Status::NOT_FOUND));
        fake.reply(FakeReply::ok(vec![0_u8; MAX_LOGO_BYTES + 1]));
        fake.reply(FakeReply::ok("<svg onload=alert(1)></svg>"));
        let source = FaviconSource::new(fake);

        for _ in 0..3 {
            assert_eq!(source.fetch(&domain()).await.unwrap(), None);
        }
    }

    #[test]
    fn image_kinds_are_told_by_their_magic_numbers() {
        assert_eq!(ImageKind::sniff(&[0, 0, 1, 0, 1]), Some(ImageKind::Ico));
        assert_eq!(ImageKind::sniff(b"GIF89a"), Some(ImageKind::Gif));
        assert_eq!(
            ImageKind::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(
            ImageKind::sniff(b"RIFF\0\0\0\0WEBPVP8 "),
            Some(ImageKind::Webp)
        );
        assert_eq!(ImageKind::sniff(b"<svg"), None);
    }

    #[tokio::test]
    async fn a_saved_logo_replaces_the_older_file_of_the_company() {
        let folder = tempfile::tempdir().unwrap();
        let files = FsLogoFiles::new(folder.path().join("logos"));
        let company = CompanyId::generate(&FakeClock::new(at("2026-10-11T09:00:00Z")));
        let ico = Logo {
            bytes: vec![0, 0, 1, 0],
            kind: ImageKind::Ico,
        };
        let png = Logo {
            bytes: PNG.to_vec(),
            kind: ImageKind::Png,
        };

        let first = files.save(&company, &ico).await.unwrap();
        let second = files.save(&company, &png).await.unwrap();

        let extension = |path: &str| {
            std::path::Path::new(path)
                .extension()
                .map(|e| e.to_string_lossy().into_owned())
        };
        assert_eq!(
            (extension(&first), extension(&second)),
            (Some("ico".into()), Some("png".into()))
        );
        assert!(!std::path::Path::new(&first).exists());
        assert_eq!(std::fs::read(&second).unwrap(), PNG);
    }
}
