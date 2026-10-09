//! The profile as system context for every LLM prompt (NOT-05).
//!
//! [`render_profile_context`] turns Settings → Profile into a short snippet;
//! [`ProfileContext`] hands it to the LLM gateway, which puts it in front of
//! every system prompt (`deino_llm::SystemContext`). So notes, chat, titles
//! and tags are written for this user: their role sets the tone, their focus
//! areas the emphasis, and their glossary fixes names and products.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::Email;
use deino_llm::SystemContext;

use crate::settings::{ProfileSettings, SettingsService};

/// The saved profile as the gateway's system context.
pub struct ProfileContext {
    settings: Arc<SettingsService>,
}

impl ProfileContext {
    /// Reads the profile from `settings` at every call, so a change applies
    /// to the next prompt.
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self { settings }
    }
}

#[async_trait]
impl SystemContext for ProfileContext {
    async fn system_context(&self) -> Option<String> {
        render_profile_context(&self.settings.get().await.profile)
    }
}

/// The system-context snippet for `profile`, or `None` for an empty profile.
pub fn render_profile_context(profile: &ProfileSettings) -> Option<String> {
    let blocks: Vec<String> = [
        facts_block(profile),
        tone_block(profile),
        labelled("Writing style", &profile.writing_style),
        labelled("Context and glossary", &profile.context),
    ]
    .into_iter()
    .flatten()
    .collect();
    if blocks.is_empty() {
        return None;
    }
    Some(blocks.join("\n\n"))
}

/// "About the user" with one line per fact they gave.
fn facts_block(profile: &ProfileSettings) -> Option<String> {
    let emails: Vec<&str> = profile.emails.iter().map(Email::as_str).collect();
    let lines: Vec<String> = [
        line("Name", &profile.name),
        line("Role", &role_at_company(profile)),
        line("Email", &emails.join(", ")),
        line("Focus areas", &profile.focus_areas.join(", ")),
    ]
    .into_iter()
    .flatten()
    .collect();
    if lines.is_empty() {
        return None;
    }
    Some(format!(
        "About the user, who these notes are for (\"Me\" in transcripts):\n{}",
        lines.join("\n")
    ))
}

/// How the role should shape the notes; the line that makes a CTO's notes
/// read differently from an account executive's.
fn tone_block(profile: &ProfileSettings) -> Option<String> {
    let role = profile.role.trim();
    if role.is_empty() {
        return None;
    }
    let focus = if profile.focus_areas.is_empty() {
        String::new()
    } else {
        " Give extra weight to anything touching their focus areas.".to_owned()
    };
    Some(format!(
        "Write for their role ({role}): lead with what someone in that role needs to know, \
         decide or act on, and use that role's vocabulary.{focus}"
    ))
}

/// "CTO at Deino", "CTO", "Works at Deino" or empty.
fn role_at_company(profile: &ProfileSettings) -> String {
    match (profile.role.trim(), profile.company.trim()) {
        ("", "") => String::new(),
        (role, "") => role.to_owned(),
        ("", company) => format!("Works at {company}"),
        (role, company) => format!("{role} at {company}"),
    }
}

/// `- Label: value`, or nothing for a blank value.
fn line(label: &str, value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| format!("- {label}: {value}"))
}

/// `Label:` and the user's free text below it, or nothing when blank.
fn labelled(label: &str, text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| format!("{label}:\n{text}"))
}

#[cfg(test)]
mod tests {
    use deino_domain::Email;

    use super::*;

    fn profile(role: &str) -> ProfileSettings {
        ProfileSettings {
            name: "Shreyas Rao".into(),
            emails: vec![Email::parse("shreyas@deino.dev").unwrap()],
            role: role.into(),
            company: "Deino".into(),
            focus_areas: vec!["Reliability".into(), "Hiring".into()],
            writing_style: "Short bullets. British spelling.".into(),
            context: "FSM = field-service management.\nBrightkite is a prospect.".into(),
        }
    }

    #[test]
    fn a_full_profile_renders_every_block() {
        insta::assert_snapshot!(render_profile_context(&profile("CTO")).unwrap());
    }

    #[test]
    fn the_role_sets_the_tone_of_the_notes() {
        let cto = render_profile_context(&profile("CTO")).unwrap();
        let seller = render_profile_context(&profile("Account executive")).unwrap();

        insta::assert_snapshot!(seller);
        assert_ne!(cto, seller);
        assert!(seller.contains("Write for their role (Account executive)"));
    }

    #[test]
    fn an_empty_profile_adds_no_context() {
        assert_eq!(render_profile_context(&ProfileSettings::default()), None);
    }

    #[test]
    fn only_what_the_user_gave_is_rendered() {
        let company_only = ProfileSettings {
            company: "Deino".into(),
            ..ProfileSettings::default()
        };

        assert_eq!(
            render_profile_context(&company_only).as_deref(),
            Some(
                "About the user, who these notes are for (\"Me\" in transcripts):\n- Role: Works at Deino"
            )
        );
    }
}
