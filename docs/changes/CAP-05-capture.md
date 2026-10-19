# CAP-05 · Speaker tags: save the call window that can't be read

## Progress log (PARITY_PLAN §7.1)
Owner QA (a real 42-minute Zoom call): the watcher reached Zoom
(`us.zoom.xos`) but logged "the zoom call window was not recognised" twice,
an intermittent AX error -25204, and "0 speaker tags saved; a call window
was not found". The adapters and fixtures were modelled by hand, and the
capture tool (`tests/capture.rs`) needs Accessibility for the terminal.
Deino already has the grant, so it can now save the real window itself.

- **Setting.** New settings section `developer` (`DeveloperSettings`,
  `crates/deino-core/src/settings/sections/developer.rs`, order 250) with
  `captureCallWindows: bool`, default `false`. Settings → Advanced →
  Developer → Diagnostics: "Save the call window when speaker names can't
  be read" (`features/developer/components/call-window-capture.tsx`).
- **Capture port.** `deino_speaker_tags::capture`: `WindowCapture`
  (`wanted`, `save(meeting, app, windows)`; no-op defaults, `NoCapture`),
  carried by `WatchParts::capture` and `SpeakerTagsParts::capture`. The
  watch (`Captures`) dumps an app at most once per meeting, on the watcher
  thread, with one full read per process: the first time its call window's
  layout is not recognised, or the first poll `CAPTURE_AFTER` (20 s) into
  the recording at which the app runs and no call window has been found
  yet. Meet dumps only Meet room windows (never other browser pages); Zoom
  and Teams dump every window. Three failed reads in a meeting stop
  further tries. `WindowReader::full_windows` reads every attribute (the
  capture tool now uses it too; `AxWindows` lost its `full` flag).
  Fakes: `FakeMac::full_reads`, `MemoryCapture`.
- **App adapter.** `composition/speaker_tags/capture.rs`
  (`LogFolderCapture`): reads `developer.captureCallWindows`, writes
  `~/Library/Logs/com.xenshreyas.deino/call-window-<app>-<meeting id>-<UTC
  time>.json` (fixture JSON, the folder from `logging::log_dir`), and logs
  one info line with the file name only.
- **Tests.** `capture_tests.rs` (once per meeting on an unknown layout and
  on the 20 s timeout, none when not wanted, none after a call window was
  found, gives up after three failed reads), the adapter's temp-dir JSON
  and `wanted` tests, and the switch's Vitest.

## Follow-ups (PARITY_PLAN §7.2)
- Owner QA: turn the setting on, have a Zoom (and Teams, Meet) call,
  anonymise the `call-window-*.json` files and add them to
  `crates/deino-speaker-tags/fixtures/`; then fix the adapters against them.
- Owner QA: the -25204 (`kAXErrorCannotComplete`) reads recover; if they
  are frequent on real calls, consider a longer messaging timeout for Zoom.

## Architecture (ARCHITECTURE.md)
- Speaker tags: "When `developer.captureCallWindows` is on, a call window
  the watch cannot read is dumped once per meeting and app
  (`WindowCapture`, `LogFolderCapture`) to the log folder in the fixture
  format, for turning into fixtures."

## CLAUDE.md
- `crates/deino-speaker-tags` row: add "the opt-in call-window dump
  (`WindowCapture`, Settings → Developer)".
