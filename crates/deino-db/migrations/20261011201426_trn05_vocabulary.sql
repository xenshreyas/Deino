-- TRN-05: custom vocabulary and the corrections it makes to transcripts.
--
-- `vocab_terms`: the user's entries in Settings → Vocabulary. A `term` is a
-- word or name spelled as given ("Deino", "PagerDuty"): any casing of it is
-- respected as written, its sounds-like variants become it, and it is listed
-- for the enhanced notes. A `replacement` is a plain rule: its variants become
-- its text, nothing else. The names of people and companies are added at
-- correction time and never stored here.
CREATE TABLE vocab_terms (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    kind TEXT NOT NULL CHECK (kind IN ('term', 'replacement')),
    text TEXT NOT NULL CHECK (length(trim(text)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- One entry per spelling and kind, whatever the case; also the list order.
CREATE UNIQUE INDEX idx_vocab_terms_kind_text ON vocab_terms (kind, text COLLATE NOCASE);

-- `vocab_variants`: what a term sounds like when the engine mishears it
-- ("Sreyas" for "Shreyas"), matched as whole words, ignoring case, in the
-- order the user listed them. An entry's variants are replaced together
-- with it, in one transaction (`VocabRepo::create` and `update`).
CREATE TABLE vocab_variants (
    term_id TEXT NOT NULL REFERENCES vocab_terms (id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    variant TEXT NOT NULL CHECK (length(trim(variant)) > 0),
    PRIMARY KEY (term_id, position)
) STRICT, WITHOUT ROWID;

-- `vocab_corrections`: the engine's original of every segment the
-- vocabulary corrected, so a correction can be undone. Written in the same
-- transaction as the corrected segment by the `vocab_fixups` stage, and right
-- after each live batch by the recorder (the live pass); a later correction
-- of the same segment keeps the first original. Undoing restores it only
-- while the segment still reads `corrected_text`, so a later edit by the user
-- is never overwritten, and stamps `undone_at`, so the stage never corrects
-- that segment again. A segment's meeting is read through
-- `transcript_segments`, whose meeting index serves the per-meeting list.
CREATE TABLE vocab_corrections (
    segment_id TEXT PRIMARY KEY NOT NULL REFERENCES transcript_segments (id) ON DELETE CASCADE,
    original_text TEXT NOT NULL,
    -- The engine's words before correction (`Vec<WordTiming>` in Rust).
    original_words_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(original_words_json) AND json_type(original_words_json) = 'array'),
    corrected_text TEXT NOT NULL,
    corrected_at TEXT NOT NULL,
    undone_at TEXT
) STRICT;
