# PRV-01 · Retention policies

## Progress log (PARITY_PLAN §7.1)
PRV-01: Settings → Privacy & security → Retention keeps recordings,
transcripts and whole meetings for never / 7 / 30 / 90 / 365 days
(`privacy.retention`, `RetentionSettings`/`RetentionPeriod`; the transcript
is validated to be kept at least as long as the audio, in Rust and in the
UI). `deino_core::retention::RetentionService::sweep` runs as `Housekeeping`
on ORG-07's `TrashPurger` loop (launch, then every 6 hours): "everything"
moves meetings to the trash, a transcript goes with its search documents and
transcript embeddings (notes kept, `meetings.transcript_deleted_at`), a
recording goes with `recordings/<id>` and its clips
(`meetings.audio_deleted_at`), through `DiskMeetingFiles::delete_recordings`,
which never deletes outside the app's folders. Plugin `retention`
(`status`); `EnhanceError::TranscriptDeleted`, the pipeline skips with
`TRANSCRIPT_DELETED`; the UI shows "Transcript deleted" and disables every
regenerate trigger (`useEnhanceAction().blocked`). The record panel shows
"Pause for sensitive discussions" once (`privacy.pauseTipSeen`).
[ADR 2026-10-12](../adr/2026-10-12-retention-runs-with-the-trash-purge.md).

## Follow-ups (PARITY_PLAN §7.2)
- TRN-02 / legacy: the inherited "open recording folder" tool still reads
  `meetings.folder_path`, which retention leaves pointing at the deleted
  folder; hide the tool when `audio_deleted_at` is set.
- TRN-02: the player could say "Recording deleted" instead of showing no
  player when `audio_deleted_at` is set (the `retention` plugin's `status`).
- AI-02 (parked, D13): chat over a meeting whose transcript was deleted
  answers from the notes only; nothing to do until chat returns.
- Owner QA: run a sweep on a real library with a short period and a copy of
  the data folder (`RUST_LOG=deino_lib=info,deino_core::retention=info`).

## Architecture (ARCHITECTURE.md)
- Crate table, `deino-core`: add `retention` (PRV-01: recordings,
  transcripts and meetings deleted after N days by a sweep on the trash
  purge's loop; ADR 2026-10-12-retention-runs-with-the-trash-purge).
- Database: `meetings.audio_deleted_at`, `meetings.transcript_deleted_at`
  (migration `20261012081356_prv01_retention.sql`).
- The app crate: `composition/retention.rs` (built from
  `composition/meetings.rs::attach_trash`, which hands the service to
  `TrashPurger::with`), `features/retention.rs` (plugin `retention`).
- Capabilities: `record-panel.json` also grants `settings:default` (the
  pause tip).

## CLAUDE.md
- Crate table, `crates/deino-core`: after `briefs (...)`, add "`retention`
  (PRV-01: recordings, transcripts and meetings deleted after N days, on the
  trash purge's loop; ADR 2026-10-12-retention-runs-with-the-trash-purge)".
- `meetings (ORG-07, ...)`: "the trash and its 30-day purge, which also runs
  retention".

## Plan updates
- PRV-01 Accept: tick "Simulated-clock tests"
  (`deino-core::retention::service_tests`, `safety_tests`) and
  "Regenerating is disabled when the transcript is gone"
  (`enhance::service_tests::retention`, `notes/components/regenerate-blocked.test.tsx`).
- §5.7 Retention row: Deino ✅ (PRV-01).
