# 0024. Store folder auto-add rules as typed rows, and auto-tag only with the user's tags

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** ORG-05 (Folders) and ORG-06 (Tags and AI auto-tagging)

## Context

FND-07 created `folders` (nested through `parent_id`), `meeting_folders`,
`tags` and `meeting_tags`, and left `folders.auto_rules_json` as a nullable
JSON placeholder that nothing wrote. ORG-05 needs folders that collect
meetings on their own: future meetings of a recurring series (Granola's
"Auto-add future meetings in this series?", research `granola` §6.2), and
meetings by tag or attendee domain. ORG-06 needs tags that the AI applies
after a meeting (Circleback's tag descriptions, research `circleback` §14)
without inventing new ones or overriding the user.

Both run after the summary, in the post-meeting pipeline (ADR 0018), and
filing by tag only works if tagging has already happened.

## Decision

- **Rules are rows.** A new migration creates `folder_rules`: one row per
  rule, `kind` CHECK-ed against `FolderRuleKind` (`series`, `tag`,
  `attendee_domain`), exactly the column of its kind set (three CHECK constraints),
  `tag_id` a foreign key to `tags` (`ON DELETE CASCADE`, so deleting a tag
  drops the rules that name it), `domain` stored lower case, and a partial
  unique index per kind so a folder holds each rule once. The migration
  drops `folders.auto_rules_json`. In Rust the rule is the tagged enum
  `deino_core::folders::FolderRule`, and `folders_matching` is a pure
  function over `MeetingFacts` (series, tag IDs, attendee domains).
- **Filing only adds.** A meeting can sit in many folders. `auto_folder`
  (order 75) files the meeting into every folder whose rules match and never
  removes a filing; the user's own moves always win. Adding a meeting from a
  recurring series by hand returns a `seriesOffer`, and the UI asks whether
  to add a `series` rule; removing that meeting from the folder removes the
  rule again. "Unfiled" is a query (meetings in no folder), not a folder row.
- **Deleting a folder keeps meetings.** Subfolders and rules go with it
  (`ON DELETE CASCADE`); meetings only leave. `FolderRepo::move_folder` is
  the only writer of `parent_id` and refuses cycles.
- **Auto-tagging chooses among existing tags.** `auto_tag` (order 70, before
  `auto_folder`, so tag rules see AI tags) sends the `tagging` route a
  prompt (askama, insta-snapshotted) with every tag that allows auto-apply,
  its description and up to three recent tagged meetings as examples, and
  the meeting's summary (else its transcript's opening, capped at 6,000
  characters). The answer is structured output (`TaggingAnswer { tags }`);
  names that are not existing tags are dropped. Tags are applied with
  `applied_by = 'ai'` and announced as `MeetingTagged`.
- **The user wins.** The stage skips when any tag on the meeting was
  applied by the user (checked again just before applying), when
  `Settings::organize.auto_tag` is off, when there are no auto-apply tags or
  no content, and, with `NoModelAutoTagger`, when no gateway could be wired.
  It never removes a tag. Skips carry a user-safe reason for the
  processing strip.

## Alternatives

- **Keep rules in `auto_rules_json`.** One column and no migration, but
  "every folder with a rule for this tag" and "drop rules of a deleted tag"
  would need JSON scans and application-side cleanup, and §3A forbids
  repeated groups in JSON for data that is queried. Rejected.
- **One column per rule kind on `folders`.** Allows one rule per kind per
  folder only, while a folder may collect several series or domains.
  Rejected.
- **Let the model create tags.** Circleback suggests new tags; the card asks
  for existing tags only, so the tag list stays the user's vocabulary.
  Suggesting new tags can be added later as a separate, confirmed action.
- **Auto-tag in the summary stage.** Saves one model call, but couples two
  routes with different models and failure handling, and NOT-02 replaces the
  summary stage. Rejected.

## Consequences

- A new rule kind is a `FolderRuleKind` variant, a column and CHECK in a new
  migration, a `FolderRule` variant and a branch in `matches`.
- Rules apply to meetings processed after they exist; a backfill of past
  meetings is a follow-up (PARITY_PLAN §7.2).
- The auto-tagger reads the legacy summary until NOT-02 lands; its
  `MeetingDigestReader` is the only place that has to change then.
- Pre-tagging upcoming calendar events (ORG-06's stretch goal) is deferred
  to ORG-02.
