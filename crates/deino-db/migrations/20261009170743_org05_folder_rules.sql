-- ORG-05: folder auto-add rules as rows (ADR 0020).
--
-- FND-07 left `folders.auto_rules_json` as an untyped, never-written
-- placeholder. Rules are queried by what they match (a series, a tag, an
-- email domain) and must vanish with the tag they name, so they get their own
-- table, one row per rule, instead of a JSON list (PARITY_PLAN §3A: no
-- repeated groups in JSON). `deino_core::folders::FolderRule` is the typed
-- model; `FolderRuleKind` mirrors the `kind` CHECK.

CREATE TABLE folder_rules (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- Deleting a folder deletes its rules.
    folder_id TEXT NOT NULL REFERENCES folders (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('series', 'tag', 'attendee_domain')),
    -- `series`: the recurring series (`meetings.series_id`) whose future
    -- meetings are filed here.
    series_id TEXT CHECK (series_id IS NULL OR length(series_id) > 0),
    -- `tag`: meetings carrying this tag. Deleting the tag deletes the rule.
    tag_id TEXT REFERENCES tags (id) ON DELETE CASCADE,
    -- `attendee_domain`: meetings with an attendee at this email domain
    -- ("acme.com"), lower case, as `Email::domain` returns it.
    domain TEXT CHECK (
        domain IS NULL OR (length(domain) > 0 AND domain = lower(trim(domain)) AND domain NOT LIKE '%@%')
    ),
    created_at TEXT NOT NULL,
    -- Exactly the column of its kind is set.
    CHECK ((kind = 'series') = (series_id IS NOT NULL)),
    CHECK ((kind = 'tag') = (tag_id IS NOT NULL)),
    CHECK ((kind = 'attendee_domain') = (domain IS NOT NULL))
) STRICT;

-- The folder page lists a folder's rules in the order they were added; the
-- folder foreign key uses the same index.
CREATE INDEX idx_folder_rules_folder_id ON folder_rules (folder_id, created_at, id);
-- A folder holds each rule once.
CREATE UNIQUE INDEX idx_folder_rules_series ON folder_rules (folder_id, series_id) WHERE series_id IS NOT NULL;
CREATE UNIQUE INDEX idx_folder_rules_tag ON folder_rules (folder_id, tag_id) WHERE tag_id IS NOT NULL;
CREATE UNIQUE INDEX idx_folder_rules_domain ON folder_rules (folder_id, domain) WHERE domain IS NOT NULL;
-- The tag foreign key: deleting a tag finds its rules (`tag_id = ?` implies NOT NULL).
CREATE INDEX idx_folder_rules_tag_id ON folder_rules (tag_id) WHERE tag_id IS NOT NULL;

-- Nothing ever wrote the placeholder; the column's own CHECK goes with it.
ALTER TABLE folders DROP COLUMN auto_rules_json;
