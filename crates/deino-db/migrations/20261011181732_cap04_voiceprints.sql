-- CAP-04: voice profiles and speaker identification.
--
-- `voiceprints`: confirmed exemplars of a person's voice. One row is the
-- centroid of a diarized speaker the user named with "rename all segments"
-- (never anything less certain: enrolment errors propagate into every later
-- meeting). `embedding` is that WeSpeaker ResNet34-LM embedding, L2-normalised,
-- as 256 little-endian f32 (1024 bytes); `embedding_model` names the model
-- (`deino_diarize::models::EMBEDDING_MODEL_ID`), so vectors of different models
-- are never compared. `quality` in [0, 1] is how much clean, overlap-free
-- speech backed the voice. A person keeps at most five exemplars: the
-- enrolment that adds a sixth evicts the lowest-quality one in the same
-- transaction (`VoiceprintRepo::enrol`, the only writer).
CREATE TABLE voiceprints (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    person_id TEXT NOT NULL REFERENCES people (id) ON DELETE CASCADE,
    embedding BLOB NOT NULL CHECK (length(embedding) = 1024),
    embedding_model TEXT NOT NULL CHECK (length(trim(embedding_model)) > 0),
    -- The meeting the voice was confirmed in; kept when that meeting goes.
    source_meeting_id TEXT REFERENCES meetings (id) ON DELETE SET NULL,
    quality REAL NOT NULL CHECK (quality BETWEEN 0.0 AND 1.0),
    created_at TEXT NOT NULL
) STRICT;

-- The FK, a person's exemplars, and eviction (lowest quality, oldest first).
CREATE INDEX idx_voiceprints_person_id ON voiceprints (person_id, quality, created_at);
-- The FK; most voiceprints keep their meeting.
CREATE INDEX idx_voiceprints_source_meeting_id ON voiceprints (source_meeting_id);

-- `speaker_suggestions`: the `identify_speakers` stage's "Is this Alex?"
-- prompts, one per speaker at most. A confirmed suggestion becomes the
-- speaker's person and the row goes; a dismissed one is deleted. The stage
-- replaces a meeting's rows each time it runs.
CREATE TABLE speaker_suggestions (
    speaker_id TEXT PRIMARY KEY NOT NULL REFERENCES speakers (id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people (id) ON DELETE CASCADE,
    -- Cosine similarity of the speaker's voice and the person's best exemplar.
    score REAL NOT NULL CHECK (score BETWEEN -1.0 AND 1.0)
) STRICT;

CREATE INDEX idx_speaker_suggestions_person_id ON speaker_suggestions (person_id);
