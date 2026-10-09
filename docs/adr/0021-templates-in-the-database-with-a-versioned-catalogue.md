# 0021. Store note templates in the database, seeded from a versioned catalogue

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** NOT-04 (Templates v2), with NOT-05 (profile) for the audience rules

## Context

Before NOT-04, summary templates were JSON files: two embedded in the binary,
six bundled as resources, and any custom file in `<app_data_dir>/templates/`
overriding a built-in of the same name, all found through two global path
statics (`summary/templates/loader.rs`). There was no editor, no way to pick
a template per meeting (`meetings.template_id` was free text, FND-07), and
no automatic choice. The card needs: an editable library, 30 immutable
built-ins that improve over releases, duplicates the user owns, auto
selection by rules then an LLM, and a picker per meeting.

## Decision

- **Rows, not files.** `templates` holds one row per template (name,
  description, meeting context, auto rules, `builtin_version`, timestamps);
  `template_sections` holds its ordered sections (`PRIMARY KEY (template_id,
  position)`, `format` CHECKed against `SectionFormat`). Sections are rows so
  the schema keeps their invariants and a template's row and sections are
  written in one transaction. Auto rules are a typed JSON column
  (`Vec<AutoRule>`): a short list evaluated in Rust (title patterns are
  regular expressions), never queried by field, as §3A allows.
- **IDs.** Built-ins have readable slugs (`one_on_one`), user templates a
  UUIDv7 in text (`TemplateId::generate`). `meetings.template_id` is now a
  foreign key (`ON DELETE SET NULL`): deleting a template returns its
  meetings to automatic selection. The migration maps the six legacy names
  to their successors (`daily_standup` → `standup`, `psychatric_session` →
  `therapy_soap`, ...) through placeholder rows.
- **Versioned built-ins.** `deino_core::templates::CATALOG` is static Rust
  data, each entry with a `version`. `BuiltinSync` runs at every start: it
  inserts missing built-ins, rewrites one whose catalogue version is newer
  than its `builtin_version`, and deletes built-ins the catalogue dropped.
  User rows have `builtin_version IS NULL`, and every write path guards on
  it, so neither a sync nor an edit can cross the line. Built-ins are
  read-only; "Duplicate" makes a user copy that upgrades never touch.
- **Selection order.** The meeting's chosen template; else the most specific
  matching auto rule (most conditions; the user's templates win ties); else
  the default template in Settings; else, only when the default is "Auto"
  and only in the post-meeting pipeline, the LLM classifier on route
  `tagging` (`TemplateClassifier`, `GatewayTemplateClassifier`); else
  General. Every choice carries a `ChoiceReason` the UI shows ("Matched rule:
  external discovery call"). Internal vs external comes from the profile's
  email domains (`profile::InternalDomains`, NOT-05).
- **Legacy JSON** files in the old folder are imported once into user
  templates and moved to `templates/imported/`; the loader, its commands and
  both statics are gone. The legacy summary reads templates through one
  adapter (`features/templates/summary_bridge.rs`).

## Alternatives

- **Sections as a JSON column.** Fewer statements per save, but no CHECK on
  the format, no non-blank title rule, and ordering by array index only.
  Rejected; the transaction cost is a handful of inserts.
- **Auto rules as rows** (`template_rules`). Normalised, but the engine reads
  every template's rules at once and compiles regexes in Rust anyway; no list
  screen filters by rule. Rejected as ceremony.
- **Seeding built-ins in SQL migrations.** Each wording change would need a
  migration, and an upgrade could not tell a built-in from a user's edit.
  Rejected for the versioned catalogue.
- **Editable built-ins with "reset to default".** Granola allows editing a
  built-in in place; we would then have to merge upgrades with edits.
  Duplicate-to-edit keeps upgrades trivially safe.
- **Calling the classifier whenever the UI shows a meeting.** It would make
  an LLM call per page view; the UI shows the rule-based choice and the
  pipeline asks the LLM once.

## Consequences

- Improving a built-in is editing `templates/builtin/*.rs` and bumping its
  `version`; `builtin_tests.rs` validates all 30.
- The classifier's answer is not stored: NOT-02 should record the template it
  generated notes with (`enhanced_notes.template_id`) and may persist an
  automatic choice.
- Templates are not shareable beyond JSON files (single user, no server).
