-- NOT-07: suggested follow-up emails.
--
-- The `follow_up_draft` stage writes one row per meeting it asked the model
-- about. Conventions as in FND-07: STRICT, NOT NULL by default, timestamps
-- RFC 3339 UTC. The primary key is the meeting; Home's "Suggested
-- follow-ups" reads ready rows by `generated_at`, hence the partial index.
--
-- JSON column (§3A): `recipients_json` (`deino_core::followups::Recipients`:
-- To, CC and BCC address lists) is part of one draft, read and replaced
-- whole and never queried by field.

-- One suggested follow-up email per meeting (NOT-07): `ready` with a subject
-- and body, `quiet` when the model decided no follow-up was needed (and
-- why), or `dismissed`, which is permanent: the stage never drafts for the
-- meeting again and the text is cleared.
CREATE TABLE follow_up_drafts (
    meeting_id TEXT PRIMARY KEY NOT NULL
        REFERENCES meetings (id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('ready', 'quiet', 'dismissed')),
    quiet_reason TEXT CHECK (quiet_reason IS NULL OR quiet_reason IN (
        'colleague_owns', 'moved_to_chat', 'blocker_first', 'nothing_to_follow_up'
    )),
    subject TEXT NOT NULL DEFAULT '' CHECK (length(subject) <= 500),
    body TEXT NOT NULL DEFAULT '' CHECK (length(body) <= 20000),
    recipients_json TEXT NOT NULL DEFAULT '{"to":[],"cc":[],"bcc":[]}'
        CHECK (json_valid(recipients_json) AND json_type(recipients_json) = 'object'),
    generated_at TEXT NOT NULL,
    -- When the user last changed the draft; an edited draft is never
    -- replaced by a pipeline re-run.
    edited_at TEXT,
    dismissed_at TEXT,
    CHECK ((status = 'quiet') = (quiet_reason IS NOT NULL)),
    CHECK ((status = 'dismissed') = (dismissed_at IS NOT NULL)),
    CHECK (status = 'ready' OR (subject = '' AND body = ''))
) STRICT;

CREATE INDEX follow_up_drafts_ready ON follow_up_drafts (generated_at)
    WHERE status = 'ready';
