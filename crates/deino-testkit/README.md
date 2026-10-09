# deino-testkit

## Role

Shared test fixtures and fakes, plus the fixture seeder behind the app's
debug-only `dev` plugin. Never part of a release build's behaviour.

## Owns

- `fixture` (FND-04): the repository's `fixtures/` as typed values.
  - `Fixtures::load()` reads `fixtures/meetings/*.json` into `FixtureMeeting`
    (serde with `deny_unknown_fields`, closed enums, a validated `Email`), then
    `validate` checks the cross-field rules: invited, attending speakers in
    order of first appearance; the user on the microphone; ordered segments
    and words inside the recording; owners who were invited; ground-truth
    `evidence` quoted from the transcript; audio files that exist.
  - `FixtureSeeder` / `seed_fixtures(&database)`: writes them through one
    `SeedTarget` per table group, in one transaction on the writer.
    `LegacyMeetingRows` and `LegacyTranscriptRows` fill `meetings` and the
    legacy `transcripts`; FND-07 adds targets for its tables.
  - `PcmTrack`: decodes the fixture audio (`mic.m4a`/`system.m4a`) into mono
    PCM frames, for FND-05's `MockAudioSource`.
- Re-exports with one import path (feature `test-support`): `test_db` (from
  `deino-db`) and `FakeClock` (from `deino-domain`).
- `at("2026-10-08T09:00:00Z")`: readable timestamps.
- `seed`: single rows no service can create yet (a meeting, a transcript chunk).

## Does Not Own

- The fixture files and their tools: `fixtures/` (see its README).
- Fakes of a crate's own ports: they live next to the trait, behind that
  crate's `test-support` feature (`FakeMeetingRepo` in `deino-core`).

## Invariants

- Depends only on `deino-domain` and `deino-db`, never on `deino-core`:
  core's tests use this crate, and a cycle would compile two copies of core's
  types.
- Fakes stay behind `test-support`. The app depends on this crate without it
  (for the `dev` plugin, compiled only with debug assertions), so release
  builds never compile a fake in.
- Fixture loading and seeding return errors instead of panicking (the `dev`
  plugin runs them in a real app); only the `seed` helpers and `at` panic.
- Seeding touches fixture rows only (`meeting-fixture-*`) and is idempotent.
