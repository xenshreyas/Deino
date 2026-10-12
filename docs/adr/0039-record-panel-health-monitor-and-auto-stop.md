# 0039. A non-activating record panel, a pure health monitor and a tick-driven auto-stop

- **Status:** Accepted
- **Date:** 2026-10-11
- **Cards:** CAP-10 (record panel, recording health, menu bar), CAP-07
  (auto-stop and meeting boundaries)

## Context

While a meeting records, the user is in another app (Zoom, Meet, a
browser). CAP-10 asks for an always-visible, draggable, edge-snapping panel
that expands on hover and **never steals focus**, a health warning within
10 s when the microphone goes silent or the transcriber dies, a menu bar
with the next meeting and "Xm left", and switching the microphone or call
audio mid-recording. CAP-07 asks for automatic stops (the call app lets go
of the microphone for 20 s, 15 minutes of silence, sleep, optionally the
calendar end), a "Start new note for …?" prompt for back-to-back meetings,
and Resume on a finished meeting. Each stop path must be unit-tested with a
simulated clock.

Two existing facts shaped the design. Detection (ADR 0023) already watches
which process holds the microphone, in `deino-detect`, with no Tauri. And
the recorder (ADR 0026) runs real-time callbacks that may not allocate, lock
or log (§3C), so health has to be read from counters the callbacks already
keep.

The `deino-recording::live_fixture` tests were also flaky under load: they
ran the host clock 8× faster than real time while the recorder measured
real-time budgets (the text holdback's 3 s hold, the pipeline's 0.5 s lag
before it pads a late track, the 2 s and 4 s rings) on that clock, so a busy
machine looked like lost audio and echo slipped through.

## Decision

1. **The panel is a non-activating `NSPanel`** (`tauri-nspanel`): it can
   never become key or main (`can_become_key_window: false`,
   `nonactivating_panel`), sits at the status level on every Space and
   accepts the first click. A transparent 380 × 176 window hosts the pill;
   outside the pill (and the back-to-back card) the window ignores the
   mouse. A 30 Hz driver thread in the app compares the cursor with the
   pill's rectangle (reported by the page through `set_panel_region`),
   toggles click-through, tells the page when the pointer is over it
   (`PanelPointer`, so hover works without focus), sends the live levels,
   snaps a dropped panel to the nearer screen edge after 12 still frames
   and remembers its position (`recording.panelPosition`, per-mille of the
   screen height). The page morphs the pill's width with a CSS grid track,
   so the window never resizes.
2. **Health is a pure monitor over counters.** `HealthMonitor` takes one
   reading a second (each track's capture status and last frame, each
   lane's VAD and transcript progress, the engines' status, free disk) and
   returns the open issues: no audio for 5 s on a track, a speaking
   microphone with no transcript for 30 s, the engine lost its stream
   (after a 10 s settle), disk under 2 GiB, and disk under 256 MiB (which
   stops the recording). A watcher thread per session applies it; the
   issues ride in `RecordingSnapshot::issues`, so the pill, the sidebar and
   notifications read one source. The audio callbacks only bump atomics
   they already had.
3. **Auto-stop is a tick-driven, pure policy in `deino-detect`**
   (`autostop::policy`), in the shape of OpenWhispr's auto-end controller
   but rewritten: each tick takes an `Observation` (paused, what each lane
   heard, whether the meeting app holds the microphone, the calendar end,
   disk full) and returns `Ending`, `Withdraw`, `Stop` or nothing. A
   release counts only after the app held the microphone for 30 s (a Siri
   request is not a call), and speech on the call restarts the wait. Stops
   other than sleep and a full disk show a 15 s "Ending…" notice with
   Keep recording; ticks more than 5 s late restart every window, so a
   stalled process never counts as silence. A recording started for a
   detected meeting follows that meeting's app; any other follows the
   meeting app that holds the microphone. Sleep stops on `NSWorkspaceWillSleepNotification`, or
   when a tick arrives more than 60 s late. The back-to-back rule
   (`autostop::boundary`) uses ORG-03's nearest-event window and offers a
   split only to an event with other invitees than the current one; a
   split stops this meeting and starts the next, linked to its event.
4. **Resume reopens the meeting's recording.** `Recorder::resume` adds a
   new manifest segment per track starting at the recorded length;
   `TrackTimeline::resuming`, the lanes, VAD and the placed-audio reader
   start at that offset, so new audio and segments follow the old ones on
   one timeline. `MeetingEvent::ResumeRecording` moves a `ready` or
   `failed` meeting back to `recording`.
5. **Mid-recording device changes go through the capture supervisor**
   (`set_microphone`, `set_system_audio`), which swaps the microphone
   source or suspends the tap without stopping the track writers; a
   disabled track records silence and the monitor ignores it.
6. **The live tests are paced by sample count.** A `PacedClock` advances
   host time only as far as the lanes have taken audio, the fixture sources
   deliver at most 500 ms ahead of it and stop exactly at the requested
   length, and the holdback waits for the Them lane's transcript watermark
   (`SessionSink::progress`) instead of a wall-clock hold (a 20 s wall-clock
   cap stays as the safety net). Padding a late track is gated on the
   capture status (only a track that is not delivering is padded).

## Consequences

- The panel cannot take keyboard input; the microphone picker is a native
  menu, and "Open notes" brings the main window forward on the record
  screen.
- Health warnings reach the UI within the watcher's 1 s period plus the
  threshold (5 s for silent audio), well inside the card's 10 s.
- Auto-stop never stops a meeting the user is talking in; the calendar rule
  is off by default. A record-only recording (CAP-08, ADR 0037) has no
  lanes listening, so it reports speech as unknown: the silence rules never
  end it, while the microphone release, sleep and a full disk still do; its
  health watcher checks audio and disk only.
- The live tests produce the same transcript on an idle and a loaded
  machine (20 of 20 identical runs at load 190–285), and the 10-minute
  ignored test runs in 70 s instead of about 2.5 minutes.
- macOS-only: the panel and the sleep notice. Elsewhere the panel is a
  plain always-on-top window and sleep is caught by the tick gap.
