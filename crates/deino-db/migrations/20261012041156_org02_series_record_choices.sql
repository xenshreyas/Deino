-- ORG-02: the user's auto-record choice for a whole recurring series ("all
-- occurrences"). One event's choice ("just this one") stays in
-- `calendar_events.record_override`; a series row applies to every
-- occurrence, including ones not synced yet, so it cannot live on the event
-- rows. Precedence (event, then series, then the global switches) is
-- `deino_core::calendar::rules::decide`.
--
-- Keyed by the provider's series key, like `calendar_events.series_id`
-- (no table owns series, so there is no foreign key). Choosing for the
-- series clears its events' own choices in the same transaction
-- (`SeriesChoiceRepo::set_series_choice`).

CREATE TABLE calendar_series_choices (
    series_id TEXT PRIMARY KEY NOT NULL CHECK (length(series_id) > 0),
    -- 1 always records, 0 never does.
    record INTEGER NOT NULL CHECK (record IN (0, 1)),
    updated_at TEXT NOT NULL
) STRICT;
