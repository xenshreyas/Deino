-- CAP-05: speaker tags, the names a meeting app's window shows.
--
-- `speaker_tag_spans`: while a meeting records, the speaker-tags watcher
-- reads the meeting app's Accessibility tree every 250 ms and stores one row
-- per stretch during which a participant's tile showed the active-speaker
-- indicator. `start_ms`/`end_ms` are offsets on the recording's own timeline
-- (pauses skipped), like `transcript_segments`. `display_name` is the name as
-- the app showed it, without role suffixes ("(Host)"); it is a label, not a
-- person: `identify_speakers` resolves it to a person or keeps it as the
-- speaker's label. `app` is the meeting app it came from. Rows are only ever
-- appended while recording (`SpeakerTagRepo::append`, the only writer) and
-- go with their meeting.
CREATE TABLE speaker_tag_spans (
    id INTEGER PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL CHECK (start_ms >= 0),
    end_ms INTEGER NOT NULL CHECK (end_ms > start_ms),
    display_name TEXT NOT NULL
        CHECK (length(trim(display_name)) > 0 AND length(display_name) <= 100),
    app TEXT NOT NULL CHECK (app IN ('zoom', 'teams', 'meet'))
) STRICT;

-- The FK, and the stage's read of one meeting's spans in time order.
CREATE INDEX idx_speaker_tag_spans_meeting_id ON speaker_tag_spans (meeting_id, start_ms);
