//! [`Automation`]: a name, when it runs and the steps it takes, and
//! [`AutomationInput`], what the builder sends and the rules it keeps.

use chrono::{DateTime, Utc};
use deino_domain::{AutomationId, ConditionGroups, MeetingFacts, TagId};
use serde::{Deserialize, Serialize};

use super::{Action, AutomationError};

/// The longest automation name, in characters (the column's CHECK).
pub const MAX_NAME_CHARS: usize = 80;
/// The most steps one automation may chain.
pub const MAX_STEPS: usize = 20;

/// A stored automation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Automation {
    /// Its ID.
    pub id: AutomationId,
    /// Its name, unique ignoring case.
    pub name: String,
    /// Off: it never runs on its own, but can still be run by hand.
    pub enabled: bool,
    /// Runs after every matching meeting is processed, and when a tag its
    /// conditions test for is applied; off: manual runs only.
    pub auto_run: bool,
    /// A failed step skips the steps after it.
    pub stop_on_failure: bool,
    /// Which meetings it runs on: AND within a group, OR across groups, no
    /// groups for every meeting.
    pub conditions: ConditionGroups,
    /// What it does, in order.
    pub steps: Vec<Action>,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

impl Automation {
    /// True when it runs on its own after meetings.
    pub const fn runs_automatically(&self) -> bool {
        self.enabled && self.auto_run
    }

    /// True when it runs on its own and a meeting with `facts` matches.
    pub fn applies_to(&self, facts: &MeetingFacts) -> bool {
        self.runs_automatically() && self.conditions.matches(facts)
    }

    /// True when applying `tag` may have made it apply (the tag trigger).
    pub fn waits_for_tag(&self, tag: &TagId) -> bool {
        self.runs_automatically() && self.conditions.mentions_tag(tag)
    }
}

/// What the builder sends to create or replace an automation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AutomationInput {
    /// Its name.
    pub name: String,
    /// See [`Automation::enabled`].
    pub enabled: bool,
    /// See [`Automation::auto_run`].
    pub auto_run: bool,
    /// See [`Automation::stop_on_failure`].
    pub stop_on_failure: bool,
    /// See [`Automation::conditions`].
    pub conditions: ConditionGroups,
    /// See [`Automation::steps`].
    pub steps: Vec<Action>,
}

impl AutomationInput {
    /// The input in canonical form: name, conditions and steps trimmed and
    /// checked; at least one step, at most [`MAX_STEPS`].
    pub fn normalized(self) -> Result<Self, AutomationError> {
        let name = self.name.trim().to_owned();
        let length = name.chars().count();
        if length == 0 || length > MAX_NAME_CHARS {
            return Err(AutomationError::invalid(
                "name",
                "must be 1 to 80 characters",
            ));
        }
        if self.steps.is_empty() {
            return Err(AutomationError::invalid("steps", "need at least one step"));
        }
        if self.steps.len() > MAX_STEPS {
            return Err(AutomationError::invalid("steps", "are limited to 20"));
        }
        let steps = self
            .steps
            .into_iter()
            .map(Action::normalized)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            name,
            conditions: self.conditions.normalized()?,
            steps,
            ..self
        })
    }

    /// The input as a new automation with `id`, created at `now`.
    pub fn into_automation(self, id: AutomationId, now: DateTime<Utc>) -> Automation {
        Automation {
            id,
            name: self.name,
            enabled: self.enabled,
            auto_run: self.auto_run,
            stop_on_failure: self.stop_on_failure,
            conditions: self.conditions,
            steps: self.steps,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::Condition;

    use super::*;

    fn input(name: &str, steps: Vec<Action>) -> AutomationInput {
        AutomationInput {
            name: name.into(),
            enabled: true,
            auto_run: true,
            stop_on_failure: false,
            conditions: ConditionGroups(vec![
                vec![],
                vec![Condition::TitleContains {
                    text: " sync ".into(),
                }],
            ]),
            steps,
        }
    }

    fn notify() -> Action {
        Action::Notify {
            message: " Done ".into(),
        }
    }

    #[test]
    fn normalising_trims_and_keeps_the_rules() {
        let clean = input("  Weekly recap ", vec![notify()])
            .normalized()
            .unwrap();

        assert_eq!(clean.name, "Weekly recap");
        assert_eq!(
            clean.steps,
            [Action::Notify {
                message: "Done".into()
            }]
        );
        assert_eq!(
            clean.conditions,
            ConditionGroups(vec![vec![Condition::TitleContains {
                text: "sync".into()
            }]])
        );
        assert!(input(" ", vec![notify()]).normalized().is_err());
        assert!(input("x", vec![]).normalized().is_err());
        assert!(
            input("x", vec![notify(); MAX_STEPS + 1])
                .normalized()
                .is_err()
        );
        let relative = Action::ExportFile {
            format: deino_export::ExportFormat::Markdown,
            folder: "notes".into(),
        };
        assert!(input("x", vec![relative]).normalized().is_err());
    }

    #[test]
    fn only_enabled_automatic_automations_run_on_their_own() {
        let now = DateTime::UNIX_EPOCH;
        let clock = deino_domain::FakeClock::new(now);
        let mut automation =
            input("x", vec![notify()]).into_automation(AutomationId::generate(&clock), now);
        automation.conditions = ConditionGroups::every_meeting();
        let facts = MeetingFacts::default();

        assert!(automation.applies_to(&facts));
        automation.auto_run = false;
        assert!(!automation.applies_to(&facts));
        automation.auto_run = true;
        automation.enabled = false;
        assert!(!automation.applies_to(&facts));
    }
}
