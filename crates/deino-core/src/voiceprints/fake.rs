//! An in-memory store for the voiceprint ports.

use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{PersonId, VoiceprintId};

use super::{Exemplar, ExemplarReader, ProfileRow, Voiceprint, VoiceprintRepo, VoiceprintSummary};
use crate::testing::lock;

/// Keeps voiceprints in a `Vec` and honours the [`ExemplarReader`] and
/// [`VoiceprintRepo`] contracts: model filtering, best-first order, eviction
/// of the weakest beyond `keep`. It knows no names: profile rows carry the
/// person's ID as their name, and only the person given to
/// [`FakeVoiceprintRepo::set_me`] is the user.
#[derive(Debug, Default)]
pub struct FakeVoiceprintRepo {
    prints: Mutex<Vec<Voiceprint>>,
    me: Mutex<Option<PersonId>>,
}

impl FakeVoiceprintRepo {
    /// Marks `person` as the user, for [`Exemplar::is_me`].
    pub fn set_me(&self, person: PersonId) {
        *lock(&self.me) = Some(person);
    }

    /// Every stored voiceprint, best first within each person.
    pub fn stored(&self) -> Vec<Voiceprint> {
        let mut prints = lock(&self.prints).clone();
        prints.sort_by(|a, b| {
            a.person_id
                .cmp(&b.person_id)
                .then(b.quality.get().total_cmp(&a.quality.get()))
                .then(b.created_at.cmp(&a.created_at))
                .then(b.id.cmp(&a.id))
        });
        prints
    }
}

#[async_trait]
impl ExemplarReader for FakeVoiceprintRepo {
    async fn exemplars(&self, model: &str) -> Result<Vec<Exemplar>, DbError> {
        let me = *lock(&self.me);
        Ok(self
            .stored()
            .into_iter()
            .filter(|print| print.embedding.model() == model)
            .map(|print| Exemplar {
                person_id: print.person_id,
                is_me: me == Some(print.person_id),
                embedding: print.embedding.values().to_vec(),
            })
            .collect())
    }
}

#[async_trait]
impl VoiceprintRepo for FakeVoiceprintRepo {
    async fn enrol(&self, print: &Voiceprint, keep: usize) -> Result<Vec<VoiceprintId>, DbError> {
        lock(&self.prints).push(print.clone());
        let surplus: Vec<VoiceprintId> = self
            .stored()
            .into_iter()
            .filter(|stored| stored.person_id == print.person_id)
            .skip(keep)
            .map(|stored| stored.id)
            .collect();
        lock(&self.prints).retain(|stored| !surplus.contains(&stored.id));
        Ok(surplus)
    }

    async fn profile_rows(&self) -> Result<Vec<ProfileRow>, DbError> {
        let me = *lock(&self.me);
        Ok(self
            .stored()
            .into_iter()
            .map(|print| ProfileRow {
                person_id: print.person_id,
                name: print.person_id.to_string(),
                is_me: me == Some(print.person_id),
                voiceprint: VoiceprintSummary {
                    id: print.id,
                    source_meeting_id: print.source_meeting_id,
                    source_meeting_title: None,
                    quality: print.quality.get(),
                    created_at: print.created_at,
                },
            })
            .collect())
    }

    async fn delete(&self, id: &VoiceprintId) -> Result<bool, DbError> {
        let mut prints = lock(&self.prints);
        let before = prints.len();
        prints.retain(|print| print.id != *id);
        Ok(prints.len() < before)
    }

    async fn forget(&self, person: &PersonId) -> Result<u64, DbError> {
        let mut prints = lock(&self.prints);
        let before = prints.len();
        prints.retain(|print| print.person_id != *person);
        Ok(u64::try_from(before - prints.len()).unwrap_or(u64::MAX))
    }
}
