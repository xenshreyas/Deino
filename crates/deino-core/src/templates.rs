//! Note templates (NOT-04): what the notes of a meeting should contain.
//!
//! - **Model.** A [`Template`] is [`TemplateContent`] (name, description,
//!   meeting context, ordered [`TemplateSection`]s with a `SectionFormat`,
//!   and [`AutoRule`]s) plus its ID and whether it is built-in. Stored in
//!   `templates` and `template_sections` (ADR 0021).
//! - **Built-ins.** [`CATALOG`] holds 30 versioned, immutable templates;
//!   [`BuiltinSync`] writes them at startup without touching the user's own.
//! - **Library.** [`TemplateService`]: list, create, edit, duplicate, delete,
//!   import and export (JSON, current or legacy format) and the one-time
//!   import of the legacy templates folder ([`LegacyTemplateFolder`]).
//! - **Per meeting.** [`MeetingTemplateService`]: which template a meeting
//!   gets and why ([`TemplateChoice`]: the user's choice, an auto rule from
//!   the pure [`RuleEngine`], the default template, the LLM
//!   [`TemplateClassifier`] on route `tagging`, or General), changing it, and
//!   previewing a template's prompt against a meeting. [`template_prompt`]
//!   is the template block of every `notes.enhance` prompt (NOT-02).

mod builtin;
mod builtin_store;
mod builtin_sync;
#[cfg(test)]
mod builtin_sync_tests;
#[cfg(test)]
mod builtin_tests;
mod classifier;
#[cfg(test)]
mod classifier_tests;
mod content;
mod engine;
#[cfg(test)]
mod engine_tests;
mod error;
mod exchange;
mod facts;
#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(test)]
mod fixture_selection_tests;
mod legacy_files;
mod meeting_service;
#[cfg(test)]
mod meeting_service_tests;
mod model;
mod prompt;
mod repo;
#[cfg(test)]
mod repo_tests;
mod rules;
mod selection;
mod service;
#[cfg(test)]
mod service_tests;

pub use builtin::{
    BuiltinRule, BuiltinSection, BuiltinTemplate, CATALOG, GENERAL_TEMPLATE_ID, successor_of_legacy,
};
pub use builtin_store::BuiltinStore;
pub use builtin_sync::{BuiltinSync, SyncReport};
pub use classifier::{Classification, GatewayTemplateClassifier, NoClassifier, TemplateClassifier};
pub use content::{
    MAX_DESCRIPTION_CHARS, MAX_INSTRUCTIONS_CHARS, MAX_MEETING_CONTEXT_CHARS, MAX_NAME_CHARS,
    MAX_SECTION_TITLE_CHARS, MAX_SECTIONS, TemplateContent, TemplateSection,
};
pub use engine::{MeetingSignals, RuleEngine, RuleMatch};
pub use error::TemplateError;
pub use exchange::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION, TemplateDocument, export_document, import_document,
};
pub use facts::{MeetingFacts, MeetingFactsReader};
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeMeetingFacts, FakeTemplateRepo, FixedClassifier};
pub use legacy_files::{IMPORTED_FOLDER, LegacyTemplateFile, LegacyTemplateFolder};
pub use meeting_service::{MeetingTemplatePorts, MeetingTemplateService};
pub use model::Template;
pub use prompt::{PromptBlock, TemplatePreview, preview, section_directive, template_prompt};
pub use repo::{SqliteTemplateRepo, TemplateRepo};
pub use rules::{
    AutoRule, CountRange, MAX_AUTO_RULES, MAX_RULE_DOMAINS, MAX_RULE_LABEL_CHARS,
    MAX_TITLE_PATTERN_CHARS,
};
pub use selection::{ChoiceReason, TemplateChoice};
pub use service::TemplateService;
