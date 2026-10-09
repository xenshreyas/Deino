-- CAP-03: each diarized speaker keeps the voice it was clustered from, so
-- CAP-04 can match it against voiceprints and enrol it on "rename all".
--
-- `centroid` is the speaker's mean WeSpeaker embedding, L2-normalised, as
-- little-endian f32 (256 values, 1024 bytes). `embedding_model` names the
-- model that produced it (`deino_diarize::models::EMBEDDING_MODEL_ID`):
-- vectors from different models are never compared. `speech_ms` is the
-- total length of the speaker's turns on the diarized track. All three are
-- set together by the diarize stage (`SpeakerRepo::replace_speakers`), the
-- only writer, and stay NULL for speakers no diarization produced (the
-- user's "Me", speakers seeded or added by hand).

ALTER TABLE speakers ADD COLUMN centroid BLOB
    CHECK (centroid IS NULL OR (length(centroid) > 0 AND length(centroid) % 4 = 0));

ALTER TABLE speakers ADD COLUMN embedding_model TEXT
    CHECK ((embedding_model IS NULL) = (centroid IS NULL));

ALTER TABLE speakers ADD COLUMN speech_ms INTEGER
    CHECK ((speech_ms IS NULL) = (centroid IS NULL) AND (speech_ms IS NULL OR speech_ms >= 0));
