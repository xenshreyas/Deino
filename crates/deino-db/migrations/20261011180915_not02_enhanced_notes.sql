-- NOT-02: enhanced notes, the AI notes of a meeting (ADR 0024).
--
-- Every generation is a new version, so regenerating never loses what was
-- there (the version history menu). A version is a write-once snapshot of
-- what the LLM wrote, except the newest one, which the user may edit in place:
-- `deino_core::enhance::SqliteEnhancedNotesRepo::save_edit` is its only
-- writer after the insert, and it sets `is_user_edited`.
--
-- `doc_json` is the TipTap document (`deino_doc::Doc`): an opaque payload
-- never queried by field (§3A). Its text runs carry the `origin` mark (user
-- or AI, and AI text the user edited) and its `citation` nodes point at
-- `transcript_segments` rows by ID; a citation of a segment deleted later
-- simply stops resolving, so there is no foreign key.
--
-- Denormalised: `markdown` is the Readable Markdown of `doc_json` (for
-- exports, search and LLM context), written in the same statement as the
-- document by the repository above. `basis_json` (`EnhanceBasis`) is a
-- fingerprint of the inputs the version was written from (attendees,
-- transcript, the user's notes), compared with the current inputs to offer
-- "regenerate" prompts; NULL for summaries imported from before NOT-02.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, an index for every foreign key.

CREATE TABLE enhanced_notes (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- 1 for the first generation, then one more per regeneration.
    version INTEGER NOT NULL CHECK (version >= 1),
    doc_json TEXT NOT NULL
        CHECK (json_valid(doc_json) AND json_extract(doc_json, '$.type') = 'doc'),
    markdown TEXT NOT NULL,
    -- The template the notes were written with; NULL once it is deleted, or
    -- for imported summaries.
    template_id TEXT REFERENCES templates (id) ON DELETE SET NULL,
    -- `provider:model` that wrote the version (`anthropic:claude-sonnet-5-5`),
    -- or `legacy` for an imported summary.
    model TEXT NOT NULL CHECK (length(model) > 0),
    basis_json TEXT CHECK (basis_json IS NULL OR json_valid(basis_json)),
    is_user_edited INTEGER NOT NULL DEFAULT 0 CHECK (is_user_edited IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, version)
) STRICT;

-- The primary key serves "the newest version" and the version list.
CREATE INDEX idx_enhanced_notes_template_id ON enhanced_notes (template_id)
    WHERE template_id IS NOT NULL;
