# deino-domain

## Role

Deino's shared vocabulary: the types that must mean the same thing in every
crate and across IPC. No I/O, no Tauri.

## Owns

- Typed IDs (`id.rs`): opaque text IDs declared with `text_id!` (`MeetingId`,
  legacy `meeting-<uuid>`; `SeriesId`; `TemplateId`, a built-in's slug or a
  UUIDv7 from `TemplateId::generate`) and UUIDv7 ID newtypes
  declared with `uuid_id!` (`SegmentId`, `PersonId`, `FolderId`, ...), each
  serde-, sqlx- and specta-transparent.
- The closed sets the core schema stores (`enums.rs`, `status.rs`), declared
  with `db_enum!` so serde, sqlx and `as_str()` agree with the column's
  `CHECK` list: `MeetingStatus` (a state machine: `apply(MeetingEvent)` and
  one method per transition), `MeetingSource`, `Channel`, `AttendeeRole`,
  `Rsvp`, `ActionItemStatus`, `ActionItemPriority`, `ActionItemOrigin`,
  `TagSource`, `CalendarProvider`, `ConferencePlatform`, `SectionFormat`
  (template sections), `Audience` (internal or external, NOT-04/NOT-05);
  and the pipeline's
  `JobStatus` and `RunOutcome` (`job.rs`, FND-08) with its `StageId`.
- `IntegrationProvider`, `AuthMethod`, `ConnectionStatus`, `Capability`,
  `DeliveryStatus` (`integration.rs`, INT-00): the integrations framework's
  closed sets, matching the CHECK lists of `integration_connections` and
  `integration_deliveries`; `ConnectionId`, `DeliveryId` in `id.rs`.
- `ServiceId` and `ServiceKind` (`service.rs`): every outside service Deino may
  talk to (FND-12), stored in `egress_log.service` and keying the settings'
  `network.services` switches. Its hosts live in `deino-net`'s registry.
- Values with rules: `Millis` and `Seconds` (`units.rs`), `Email` (`email.rs`,
  canonical lower-case addresses; `parse` normalises user input), and
  `PaletteColor` (`color.rs`, a slot of the shared speaker and tag palette).
- The IPC error model (`error.rs`): `AppError { code, message }`, `ErrorCode`,
  and the `ErrorClass` trait every module error implements (ADR 0009).
- The `Clock` port (`clock.rs`): `SystemClock`, and `FakeClock` behind the
  `test-support` feature.
- The `LocalZone` port (`zone.rs`, ADR 0028): the user's time zone for local
  days (`day_of`, `at`, `start_of`); `SystemZone` over the Mac's zone and
  `FixedZone` for tests. Due days are stored as `NOON` local.
- `Condition` and `ConditionGroups` (`conditions.rs`, AI-06, shared with
  AUT-01): which meetings something applies to, and `MeetingFacts`, what
  they test; pure evaluation (AND within a group, OR across, none = every
  meeting) and `normalized`; AUT-01's kinds (participants, title is, is not
  and regex, invitee emails and count, domains all or not all, duration,
  calendar) are here too. `InsightId` (`id.rs`), `InsightStatus` and
  `InsightApplyMode` (`enums.rs`); `AutomationId` and `AutomationRunId`,
  and `AutomationRunStatus`, `AutomationStepStatus` and `AutomationTrigger`
  (`automation.rs`, AUT-01).

## Does Not Own

- Feature models and rules: `deino-core`.
- Database access or the `Table` enum: `deino-db`.
- Anything Tauri: the app crate.

## Invariants

- No dependency on `tauri`, `deino-db` or `deino-core`.
- New entity IDs are declared in `id.rs` with `uuid_id!` and generated from a
  `Clock`, never `Uuid::now_v7()`.
- `ErrorCode` is a closed set; adding a code changes the bindings and the
  UI's handling, so it needs a reason in the PR.
- `AppError::message` is user-safe English: never meeting content.
- `SystemClock` is the only code allowed to read the wall clock.
- A `db_enum!` value is renamed or added only together with a migration that
  changes its column's `CHECK` list.
