//! The logo rules: fetched once, refusals not recorded, refresh on demand.

use std::sync::Mutex;

use deino_domain::ServiceId;

use super::*;
use crate::entities::testing::World;

/// Answers each fetch with the next scripted result and counts fetches.
#[derive(Default)]
struct ScriptedSource {
    answers: Mutex<Vec<Result<Option<Logo>, LogoError>>>,
    fetched: Mutex<usize>,
}

impl ScriptedSource {
    fn then(self, answer: Result<Option<Logo>, LogoError>) -> Self {
        self.answers.lock().unwrap().insert(0, answer);
        self
    }

    fn fetched(&self) -> usize {
        *self.fetched.lock().unwrap()
    }
}

#[async_trait]
impl LogoSource for ScriptedSource {
    async fn fetch(&self, _domain: &CompanyDomain) -> Result<Option<Logo>, LogoError> {
        *self.fetched.lock().unwrap() += 1;
        self.answers.lock().unwrap().pop().unwrap_or(Ok(None))
    }
}

/// Stores nothing; names the file it would have written.
struct NamedFiles;

#[async_trait]
impl LogoFiles for NamedFiles {
    async fn save(&self, company: &CompanyId, logo: &Logo) -> Result<String, LogoError> {
        Ok(format!("/logos/{company}.{}", logo.kind.extension()))
    }
}

fn png() -> Logo {
    Logo {
        bytes: vec![0x89, b'P', b'N', b'G'],
        kind: ImageKind::Png,
    }
}

fn service(world: &World, source: Arc<ScriptedSource>) -> LogoService {
    let ports = world.ports();
    LogoService::new(LogoPorts {
        companies: ports.companies,
        directory: ports.directory,
        writes: ports.writes,
        source,
        files: Arc::new(NamedFiles),
        clock: ports.clock,
    })
}

#[tokio::test]
async fn a_logo_is_fetched_once_and_cached() {
    let world = World::new().await;
    let acme = world.company("Acme", Some("acme.com")).await;
    let source = Arc::new(ScriptedSource::default().then(Ok(Some(png()))));
    let logos = service(&world, source.clone());

    let first = logos.ensure(&acme.id, false).await.unwrap();
    let second = logos.ensure(&acme.id, false).await.unwrap();

    let expected = format!("/logos/{}.png", acme.id);
    assert_eq!(first.as_deref(), Some(expected.as_str()));
    assert_eq!(second, first);
    assert_eq!(source.fetched(), 1);
    let stored = world.stored_company(&acme.id).await;
    assert_eq!(stored.logo_path, first);
    assert!(stored.logo_checked_at.is_some());
}

#[tokio::test]
async fn a_refused_fetch_is_not_recorded_so_it_runs_once_allowed() {
    let world = World::new().await;
    let acme = world.company("Acme", Some("acme.com")).await;
    let refused = LogoError::Network(NetError::ServiceDisabled(ServiceId::Favicons));
    let source = Arc::new(
        ScriptedSource::default()
            .then(Err(refused))
            .then(Ok(Some(png()))),
    );
    let logos = service(&world, source.clone());

    let error = logos.ensure(&acme.id, false).await.unwrap_err();
    assert!(matches!(&error, EntityError::Logo(error) if error.is_refused()));
    assert_eq!(world.stored_company(&acme.id).await.logo_checked_at, None);

    assert!(logos.ensure(&acme.id, false).await.unwrap().is_some());
    assert_eq!(source.fetched(), 2);
}

#[tokio::test]
async fn an_unreachable_site_counts_as_asked_until_a_refresh() {
    let world = World::new().await;
    let acme = world.company("Acme", Some("acme.com")).await;
    let down = LogoError::Network(NetError::Redirect);
    let source = Arc::new(
        ScriptedSource::default()
            .then(Err(down))
            .then(Ok(Some(png()))),
    );
    let logos = service(&world, source.clone());

    assert_eq!(logos.ensure(&acme.id, false).await.unwrap(), None);
    assert_eq!(logos.ensure(&acme.id, false).await.unwrap(), None);
    assert!(logos.ensure(&acme.id, true).await.unwrap().is_some());
    assert_eq!(source.fetched(), 2);
}

#[tokio::test]
async fn a_company_without_a_domain_has_nothing_to_fetch() {
    let world = World::new().await;
    let local = world.company("Corner Bakery", None).await;
    let source = Arc::new(ScriptedSource::default());

    let logo = service(&world, source.clone())
        .ensure(&local.id, true)
        .await
        .unwrap();

    assert_eq!((logo, source.fetched()), (None, 0));
}
