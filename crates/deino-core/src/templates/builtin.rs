//! The built-in template catalogue: 30 templates Deino ships (NOT-04).
//!
//! Each [`BuiltinTemplate`] is static data with a stable slug ID and a
//! `version`. `BuiltinSync` writes the catalogue into the database at
//! startup: it adds new built-ins, rewrites one whose catalogue version is
//! newer than the stored one, and removes built-ins the catalogue dropped. It
//! never touches the user's own templates, including duplicates of
//! built-ins, so improving a built-in here (and bumping its `version`) is
//! safe. The instructions drive note quality: keep them specific, concise
//! and about the meeting's content, never the output's Markdown syntax
//! (the section's format covers that).

mod advisory;
mod customers;
mod hiring;
mod leadership;
mod learning;
mod team;
mod workshops;

use deino_domain::{Audience, SectionFormat, TemplateId};

use super::{AutoRule, CountRange, TemplateContent, TemplateSection};

/// The ID of the General template, the fallback when nothing else is chosen.
pub const GENERAL_TEMPLATE_ID: &str = "general";

/// A template shipped with Deino.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinTemplate {
    /// The stable slug stored as `templates.id`.
    pub id: &'static str,
    /// Bump it whenever anything below changes, so existing installs upgrade.
    pub version: u32,
    /// The name in pickers.
    pub name: &'static str,
    /// One line on what it is for.
    pub description: &'static str,
    /// Who the meeting is with and what the notes are for.
    pub meeting_context: &'static str,
    /// The sections, in order.
    pub sections: &'static [BuiltinSection],
    /// When to choose it automatically.
    pub rules: &'static [BuiltinRule],
}

/// A section of a built-in template.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinSection {
    /// The heading.
    pub title: &'static str,
    /// How it is rendered.
    pub format: SectionFormat,
    /// What goes in it.
    pub instructions: &'static str,
}

/// An auto rule of a built-in template (built-ins never match on domains).
#[derive(Clone, Copy, Debug)]
pub struct BuiltinRule {
    /// What the user sees when it matched.
    pub label: &'static str,
    /// A case-insensitive title pattern.
    pub title: Option<&'static str>,
    /// Internal or external.
    pub audience: Option<Audience>,
    /// The inclusive attendee count range.
    pub attendees: Option<(u32, u32)>,
}

/// A section, for the catalogue's tables.
pub(super) const fn section(
    title: &'static str,
    format: SectionFormat,
    instructions: &'static str,
) -> BuiltinSection {
    BuiltinSection {
        title,
        format,
        instructions,
    }
}

impl BuiltinRule {
    /// A rule matching titles against `pattern`.
    pub(super) const fn titled(label: &'static str, pattern: &'static str) -> Self {
        Self {
            label,
            title: Some(pattern),
            audience: None,
            attendees: None,
        }
    }

    /// A rule with no title pattern.
    pub(super) const fn untitled(label: &'static str) -> Self {
        Self {
            label,
            title: None,
            audience: None,
            attendees: None,
        }
    }

    /// Also requires someone from outside the user's organisation.
    pub(super) const fn external(self) -> Self {
        Self {
            audience: Some(Audience::External),
            ..self
        }
    }

    /// Also requires colleagues only.
    pub(super) const fn internal(self) -> Self {
        Self {
            audience: Some(Audience::Internal),
            ..self
        }
    }

    /// Also requires between `min` and `max` attendees.
    pub(super) const fn attendees(self, min: u32, max: u32) -> Self {
        Self {
            attendees: Some((min, max)),
            ..self
        }
    }

    fn to_rule(self) -> AutoRule {
        AutoRule {
            label: self.label.to_owned(),
            title_pattern: self.title.map(str::to_owned),
            attendee_domains: Vec::new(),
            audience: self.audience,
            attendee_count: self.attendees.map(|(min, max)| CountRange {
                min: Some(min),
                max: Some(max),
            }),
        }
    }
}

impl BuiltinTemplate {
    /// Its database ID.
    pub fn template_id(&self) -> TemplateId {
        TemplateId::new(self.id)
    }

    /// Its content, as stored.
    pub fn content(&self) -> TemplateContent {
        TemplateContent {
            name: self.name.to_owned(),
            description: self.description.to_owned(),
            meeting_context: self.meeting_context.to_owned(),
            sections: self
                .sections
                .iter()
                .map(|section| TemplateSection {
                    title: section.title.to_owned(),
                    instructions: section.instructions.to_owned(),
                    format: section.format,
                })
                .collect(),
            auto_rules: self.rules.iter().map(|rule| rule.to_rule()).collect(),
        }
    }
}

/// Every built-in, in the order pickers show them.
pub const CATALOG: [BuiltinTemplate; 30] = [
    team::GENERAL,
    workshops::CIRCLEBACK_STYLE,
    team::ONE_ON_ONE,
    team::STANDUP,
    team::WEEKLY_TEAM_SYNC,
    team::PROJECT_KICKOFF,
    team::SPRINT_PLANNING,
    team::RETROSPECTIVE,
    workshops::DESIGN_CRITIQUE,
    workshops::BRAINSTORM,
    workshops::ALL_HANDS,
    leadership::BOARD_MEETING,
    leadership::INVESTOR_UPDATE,
    leadership::INVESTOR_PITCH,
    customers::SALES_DISCOVERY,
    customers::SALES_DEMO,
    customers::PIPELINE_REVIEW,
    customers::CUSTOMER_CHECK_IN,
    customers::CUSTOMER_SUPPORT,
    hiring::USER_INTERVIEW,
    hiring::CANDIDATE_INTERVIEW,
    hiring::INTERVIEW_DEBRIEF,
    learning::LECTURE,
    learning::WEBINAR,
    learning::PODCAST,
    hiring::COACHING_SESSION,
    advisory::THERAPY_SOAP,
    advisory::LEGAL_CONSULTATION,
    advisory::PARTNERSHIP_CALL,
    advisory::VENDOR_EVALUATION,
];

/// The NOT-04 successor of a pre-NOT-04 JSON template ID (the same mapping
/// the NOT-04 migration applies to `meetings.template_id`).
pub fn successor_of_legacy(id: &str) -> Option<&'static str> {
    Some(match id {
        "standard_meeting" => GENERAL_TEMPLATE_ID,
        "daily_standup" => team::STANDUP.id,
        "project_sync" => team::WEEKLY_TEAM_SYNC.id,
        "retrospective" => team::RETROSPECTIVE.id,
        "sales_marketing_client_call" => customers::CUSTOMER_CHECK_IN.id,
        "psychatric_session" => advisory::THERAPY_SOAP.id,
        _ => return None,
    })
}
