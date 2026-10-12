-- AI-04 and AI-02-lite (D13): recipes, and the "Ask about this meeting" thread.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, booleans as 0/1 with a CHECK.

-- Recipes (AI-04): saved prompts the user runs on a meeting, by `/slug` in
-- the ask bar or from the Recipes page. Built-in rows (`built_in` = 1) are
-- written from the app's library (`deino_core::recipes::LIBRARY`) and kept
-- in step with it; the user's own rows are theirs to edit. `kind` says what a
-- recipe runs on: `single` (one meeting) or `multi` (several meetings; the
-- built-ins of that kind are listed as WIP until cross-meeting chat returns).
-- `slug` is the `/shortcut`: lowercase words joined by hyphens, unique.
CREATE TABLE recipes (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE
        CHECK (length(slug) BETWEEN 1 AND 48 AND slug NOT GLOB '*[^a-z0-9-]*'),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    prompt TEXT NOT NULL CHECK (length(trim(prompt)) > 0),
    kind TEXT NOT NULL CHECK (kind IN ('single', 'multi')),
    built_in INTEGER NOT NULL DEFAULT 0 CHECK (built_in IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- One meeting's "Ask about this meeting" thread (AI-02-lite, D13): the
-- user's questions and the cited answers, oldest first, a short history the
-- service prunes. JSON column (§3A): `parts_json` (`Vec<AnswerPart>`) is a
-- write-once snapshot of one message (text runs, each with the transcript
-- segments it cites), read whole and never queried by field. Citations name
-- segments by ID without foreign keys, like enhanced notes' citations: a
-- citation of a segment deleted later stops resolving. `recipe_slug` and
-- `recipe_name` label a question a recipe asked; no foreign key, so the
-- label outlives the recipe.
CREATE TABLE ask_messages (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    parts_json TEXT NOT NULL
        CHECK (json_valid(parts_json) AND json_type(parts_json) = 'array'),
    recipe_slug TEXT,
    recipe_name TEXT,
    created_at TEXT NOT NULL,
    CHECK ((recipe_slug IS NULL) = (recipe_name IS NULL))
) STRICT;

CREATE INDEX ask_messages_by_meeting ON ask_messages (meeting_id, created_at);
