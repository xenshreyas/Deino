//! Wiring for speaker naming and voice profiles (CAP-04): the services the
//! `speakers` plugin calls, and the identifier the `identify_speakers`
//! pipeline stage runs (with CAP-05's speaker tags as priors), all over the
//! SQLite repositories.

use std::sync::Arc;

use deino_core::companies::SqliteCompanyRepo;
use deino_core::diarization::{IdentifyPolicy, IdentifyPorts, IdentifyService, SpeakerIdentifier};
use deino_core::entities::EntityDirectory;
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::speakers::{SpeakerPorts, SpeakerService, SqliteNamingRepo};
use deino_core::voiceprints::{SqliteVoiceprintRepo, VoiceprintDeps, VoiceprintService};
use deino_db::Database;
use deino_domain::{Clock, SystemClock};
use tauri::{Manager, Runtime};

use super::attach::Phase;

super::attach::attach!(
    "speakers",
    10,
    Phase::Local(|cx| {
        Speakers::build(cx.database, cx.clock).manage(cx.app);
    })
);

/// The speaker services, built once over one database.
pub(crate) struct Speakers {
    speakers: Arc<SpeakerService>,
    voiceprints: Arc<VoiceprintService>,
}

impl Speakers {
    /// Builds the naming and voice-profile services over `database`.
    pub(crate) fn build(database: &Database, clock: &Arc<dyn Clock>) -> Self {
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let prints = Arc::new(SqliteVoiceprintRepo::new(database.clone()));
        let naming = Arc::new(SqliteNamingRepo::new(database.clone()));
        let voiceprints = Arc::new(VoiceprintService::new(VoiceprintDeps {
            repo: prints.clone(),
            speakers: segments.clone(),
            segments: segments.clone(),
            clock: Arc::clone(clock),
        }));
        let speakers = Arc::new(SpeakerService::new(SpeakerPorts {
            participants: naming.clone(),
            naming: naming.clone(),
            speakers: segments.clone(),
            people: people.clone(),
            attendees: people.clone(),
            voiceprints: Arc::clone(&voiceprints),
            directory: Arc::new(EntityDirectory::new(
                people.clone(),
                Arc::new(SqliteCompanyRepo::new(database.clone())),
                Arc::clone(clock),
            )),
            clock: Arc::clone(clock),
        }));
        Self {
            speakers,
            voiceprints,
        }
    }

    /// Registers the services the `speakers` plugin's commands take.
    pub(crate) fn manage<R: Runtime>(&self, app: &impl Manager<R>) {
        app.manage(Arc::clone(&self.speakers));
        app.manage(Arc::clone(&self.voiceprints));
    }
}

/// The `identify_speakers` stage's identifier over `database`, with
/// thresholds from `policy` (the settings in the app) and the meeting app's
/// speaker tags as priors (CAP-05).
pub(crate) fn identifier(
    database: &Database,
    policy: Arc<dyn IdentifyPolicy>,
) -> Arc<dyn SpeakerIdentifier> {
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let ports = IdentifyPorts {
        speakers: Arc::new(SqliteSegmentRepo::new(database.clone())),
        attendees: people.clone(),
        me: people,
        exemplars: Arc::new(SqliteVoiceprintRepo::new(database.clone())),
    };
    let writer = Arc::new(SqliteNamingRepo::new(database.clone()));
    let tags = super::speaker_tags::tag_priors(database, Arc::new(SystemClock));
    Arc::new(IdentifyService::new(ports, policy, writer).with_speaker_tags(tags))
}

#[cfg(test)]
mod tests {
    use deino_core::diarization::Identified;
    use deino_core::settings::IdentifyThresholds;
    use deino_domain::MeetingId;
    use deino_testkit::{FakeClock, at, seed, test_db};

    use super::*;

    struct Off;

    #[async_trait::async_trait]
    impl IdentifyPolicy for Off {
        async fn thresholds(&self) -> Option<IdentifyThresholds> {
            None
        }
    }

    #[tokio::test]
    async fn the_services_share_the_database_they_are_built_over() {
        let database = test_db().await;
        seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-11T09:00:00Z"),
        )
        .await;
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-11T10:00:00Z")));

        let speakers = Speakers::build(&database, &clock);

        let meeting = MeetingId::new("meeting-1");
        assert!(
            speakers
                .speakers
                .participants(&meeting)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(speakers.voiceprints.profiles().await.unwrap().is_empty());
        let identified = identifier(&database, Arc::new(Off))
            .identify(&meeting)
            .await;
        assert_eq!(identified.unwrap(), Identified::Disabled);
    }
}
