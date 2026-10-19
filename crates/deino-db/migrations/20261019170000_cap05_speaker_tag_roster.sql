-- CAP-05: speaker tags' remote roster, who the meeting app's call window listed.
--
-- `speaker_tag_roster`: while a meeting records, the speaker-tags watcher
-- reads the call window every 250 ms and stores one row per stretch during
-- which a remote participant (never the user) was listed in it, speaking or
-- not. Zoom shows no talking indicator, so the roster is what tells a
-- one-to-one call from a group call: `identify_speakers` names every remote
-- voice of a call with one remote person throughout after that person.
-- Columns as in `speaker_tag_spans`; rows are only ever appended while
-- recording (`SpeakerTagWriter::append_roster`, the only writer) and go with
-- their meeting.
CREATE TABLE speaker_tag_roster (
    id INTEGER PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL CHECK (start_ms >= 0),
    end_ms INTEGER NOT NULL CHECK (end_ms > start_ms),
    display_name TEXT NOT NULL
        CHECK (length(trim(display_name)) > 0 AND length(display_name) <= 100),
    app TEXT NOT NULL CHECK (app IN ('zoom', 'teams', 'meet'))
) STRICT;

-- The FK, and the stage's read of one meeting's roster in time order.
CREATE INDEX idx_speaker_tag_roster_meeting_id ON speaker_tag_roster (meeting_id, start_ms);
