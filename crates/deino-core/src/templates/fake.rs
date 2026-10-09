//! In-memory template ports for tests, honouring the SQLite contracts
//! (ordering by name, built-ins untouchable through `TemplateRepo`).

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, TemplateId};
use deino_llm::LlmError;

use super::{
    BuiltinStore, Classification, MeetingFacts, MeetingFactsReader, Template, TemplateClassifier,
    TemplateRepo,
};
use crate::testing::lock;

/// Templates in a `Vec`, with the catalogue version of each built-in.
#[derive(Debug, Default)]
pub struct FakeTemplateRepo {
    templates: Mutex<Vec<(Template, Option<u32>)>>,
}

impl FakeTemplateRepo {
    fn write<T>(&self, change: impl FnOnce(&mut Vec<(Template, Option<u32>)>) -> T) -> T {
        change(&mut lock(&self.templates))
    }
}

#[async_trait]
impl TemplateRepo for FakeTemplateRepo {
    async fn list(&self) -> Result<Vec<Template>, DbError> {
        let mut templates: Vec<Template> = lock(&self.templates)
            .iter()
            .map(|(t, _)| t.clone())
            .collect();
        templates
            .sort_by_key(|template| (template.content.name.to_lowercase(), template.id.clone()));
        Ok(templates)
    }

    async fn get(&self, id: &TemplateId) -> Result<Option<Template>, DbError> {
        Ok(lock(&self.templates)
            .iter()
            .find(|(t, _)| t.id == *id)
            .map(|(t, _)| t.clone()))
    }

    async fn insert(&self, template: &Template) -> Result<(), DbError> {
        self.write(|templates| templates.push((template.clone(), None)));
        Ok(())
    }

    async fn update(&self, template: &Template) -> Result<bool, DbError> {
        Ok(self.write(|templates| {
            let stored = templates
                .iter_mut()
                .find(|(t, version)| t.id == template.id && version.is_none());
            stored
                .map(|(stored, _)| *stored = template.clone())
                .is_some()
        }))
    }

    async fn delete(&self, id: &TemplateId) -> Result<bool, DbError> {
        Ok(self.write(|templates| {
            let before = templates.len();
            templates.retain(|(t, version)| !(t.id == *id && version.is_none()));
            templates.len() < before
        }))
    }
}

#[async_trait]
impl BuiltinStore for FakeTemplateRepo {
    async fn builtin_versions(&self) -> Result<Vec<(TemplateId, u32)>, DbError> {
        let templates = lock(&self.templates);
        Ok(templates
            .iter()
            .filter_map(|(t, version)| Some((t.id.clone(), (*version)?)))
            .collect())
    }

    async fn save_builtin(&self, template: &Template, version: u32) -> Result<(), DbError> {
        self.write(
            |templates| match templates.iter_mut().find(|(t, _)| t.id == template.id) {
                Some((_, None)) => {}
                Some((stored, stored_version)) => {
                    let created_at = stored.created_at;
                    *stored = Template {
                        created_at,
                        ..template.clone()
                    };
                    *stored_version = Some(version);
                }
                None => templates.push((template.clone(), Some(version))),
            },
        );
        Ok(())
    }

    async fn remove_builtin(&self, id: &TemplateId) -> Result<bool, DbError> {
        Ok(self.write(|templates| {
            let before = templates.len();
            templates.retain(|(t, version)| !(t.id == *id && version.is_some()));
            templates.len() < before
        }))
    }
}

/// Meeting facts by meeting ID.
#[derive(Debug, Default)]
pub struct FakeMeetingFacts {
    meetings: Mutex<HashMap<MeetingId, MeetingFacts>>,
}

impl FakeMeetingFacts {
    /// Adds or replaces a meeting's facts.
    pub fn insert(&self, facts: MeetingFacts) {
        lock(&self.meetings).insert(facts.id.clone(), facts);
    }
}

#[async_trait]
impl MeetingFactsReader for FakeMeetingFacts {
    async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError> {
        Ok(lock(&self.meetings).get(meeting).cloned())
    }
}

/// A classifier that always answers the same.
#[derive(Debug, Default)]
pub struct FixedClassifier(pub Option<Classification>);

#[async_trait]
impl TemplateClassifier for FixedClassifier {
    async fn classify(
        &self,
        _meeting: &MeetingFacts,
        _candidates: &[Template],
    ) -> Result<Option<Classification>, LlmError> {
        Ok(self.0.clone())
    }
}
