-- AI-01: the semantic index (PARITY_PLAN §4.4, ADR 0032).
--
-- One row per embedded chunk: a window of the transcript (about 400 tokens,
-- cut at speaker turns, overlapping by one turn) or a section of the user's
-- notes or of the AI's enhanced notes. Vectors are little-endian f32 BLOBs, unit length,
-- searched by an in-memory cosine index in `deino_core::search::semantic`
-- (ADR 0032 explains why not the sqlite-vec extension).
--
-- Denormalised on purpose (§3A): `text` copies the chunk's source text, so
-- results can show it and chat (AI-02) can quote it without re-reading the
-- transcript. Source of truth: `transcript_segments`, `meeting_notes` and
-- `enhanced_notes` (the newest version). The one writer is `SqliteEmbeddingStore::replace`
-- (the `index` pipeline stage and the backfill), which replaces a meeting's
-- chunks in one transaction and reuses the vector of every chunk whose text
-- and model are unchanged.

CREATE TABLE embeddings (
    -- The in-memory index's handle on the row.
    id INTEGER PRIMARY KEY,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    ref_kind TEXT NOT NULL CHECK (ref_kind IN ('transcript', 'note', 'summary')),
    -- The window's first segment for `transcript`; the meeting for a section.
    ref_id TEXT NOT NULL CHECK (length(ref_id) > 0),
    -- The chunk's position within the meeting's chunks of `ref_kind`.
    chunk_ix INTEGER NOT NULL CHECK (chunk_ix >= 0),
    -- The window's span in the recording; NULL for note and summary sections.
    ts_ms INTEGER CHECK (ts_ms IS NULL OR ts_ms >= 0),
    end_ms INTEGER CHECK (end_ms IS NULL OR (ts_ms IS NOT NULL AND end_ms >= ts_ms)),
    text TEXT NOT NULL CHECK (length(text) > 0),
    -- The embedding model that wrote `vector`; another model means re-embed.
    model TEXT NOT NULL CHECK (length(model) > 0),
    vector BLOB NOT NULL CHECK (length(vector) > 0 AND length(vector) % 4 = 0),
    created_at TEXT NOT NULL,
    -- Also serves the foreign key and "this meeting's chunks".
    UNIQUE (meeting_id, ref_kind, chunk_ix)
) STRICT;

-- Settings' coverage ("412 of 430 meetings") per model, without reading a
-- single vector (each row carries about 3 KB of them).
CREATE INDEX idx_embeddings_model_meeting_id ON embeddings (model, meeting_id);
