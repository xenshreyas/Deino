# ORG-07 handoff (agent A → agent B)

Branch `feat/ORG-07-meetings-list`, pushed, not merged. Phase A is done.

## Done
- **Migration** `20261012003435_org07_meetings_list.sql`: `meeting_views` (saved filters, JSON of the typed `MeetingFilter`, like AI-05) and `meeting_merge_parts` (each absorbed meeting's audio paths + offset). No new filter indexes: the page query walks `idx_meetings_live_created_at_id` (EXPLAIN test).
- **Core** `crates/deino-core/src/meetings/`: `list/` (typed `MeetingFilter`, cursor pages, `matching_ids` for ⌘A; OR within a facet, AND across; folders include subfolders; people = attendee or linked speaker; internal/external from `profile::InternalDomains`), `views.rs` + `view_service.rs`, `trash/` (batch trash/restore, Trash list with `purgesAt`, delete forever, `TrashPurger` every 6 h deleting meetings trashed > 30 days plus `recordings/<id>` and `meetings/<id>` via `DiskMeetingFiles`, never files outside the app folders), `merge/` (one transaction: segments + word timings offset, speakers joined by person/Me, attendance with the stronger role, tags/folders/action items/voiceprints moved, notes combined under part headings, parts recorded, `duration_s` = timeline), `bulk.rs` (tag/untag/file through `TagService`/`FilingService`).
- **`MeetingRepo::find` now skips trashed meetings**, so the meeting page, exports, the CLI (`deino-core::query`) and pipeline stages stop seeing them. Search/entities/related already filtered `deleted_at`.
- **Settings** `ui.meetingsList { view, filter }` persists the page.
- **Plugin** `meetings`: `page, matching_ids, views, create_view, update_view, delete_view, trash, restore, trashed, delete_forever, tag_many, untag_many, file_many, merge`; contract tests in `features/meetings/list_contract_tests.rs`; wiring in `composition/meetings.rs` (purge stopped in `lifecycle.rs`).
- **Legacy delete removed** (`api_delete_meeting`, `storageService`); every delete is now a trash.
- **UI** `frontend/src/features/meetings-list/` (route `/meetings`, new `/meetings/trash`, sidebar "Trash" in nav.ts). Old `features/meetings` screen/rows/dialogs deleted. dev:mock answers all commands (`dev/mock-ipc/mock-meetings-list.ts`).
- **Tests**: 78 Rust tests under `meetings::` + 8 contract tests; Vitest `model.test.ts`, `meetings-list-screen.test.tsx`. Ignored benchmark `filtering_2000_meetings_takes_under_150_ms` (passes in debug at load 40: ~0.2 s total incl. seeding).

## Left for agent B
1. **Playwright** over dev:mock: list grouped by day, F filter menu, save view tab, select + Shift + ⌘A, bulk tag/folder/delete + Undo, Trash restore/delete forever, merge → opens merged meeting. Existing specs that touched the old page may need updating: `e2e/{home,folders,meeting,shell,tags}.spec.ts`, `e2e/fixtures.ts` (row menu no longer has Rename; Delete now trashes).
2. **Visual polish vs `docs/references/ui.md` §4.4**, light + dark screenshots (row density, avatar overlap, bulk bar width at narrow windows, Trash row buttons).
3. **Benchmark**: `cargo test -p deino-core --release --lib filtering_2000 -- --ignored` (record machine load).
4. **Full gate** `scripts/ci-local.sh`.
5. **Docs**: ARCHITECTURE.md (meetings modules, two tables, purge loop), CLAUDE.md crate table, PARITY_PLAN progress log + §7.2 rows (resolved: FND-13/FND-04/FND-10 legacy delete, NOT-01 images folder, ORG-06 bulk tagging, ORG-04 merge attendance), maybe an ADR (merge parts + trash rules).

## Gaps / follow-ups (not built)
- Bulk **export** is enabled for exactly one meeting (opens `ExportDialog`); many-at-once needs a batch export in EXP's `ExportService`.
- `untag_many` has no UI yet. "Run automation" is a disabled WIP button (AUT-01).
- FND-08 follow-up still open: cancel a meeting's pipeline when it is trashed.
- After a merge, FTS follows by triggers, but **embeddings of the merged meeting are stale** until its `index` stage reruns; the **player does not read `meeting_merge_parts`** yet (TRN-02).
- dev:mock ignores the "has action items" and internal/external filters.
- A merged speaker whose label clashes gets ` (N)`; a second clash would fail the merge (rare).

## Gotchas
- New/changed SQL needs `scripts/sqlx-prepare.sh` (offline `.sqlx/`). `Table::ALL` is now 34, so `bindings.rs` serializes `Table::ALL.as_slice()`.
- `tauri-build` needs sidecar stubs in a fresh worktree (`scripts/ci-local.sh` stages them, or copy the two scripts from main's `frontend/src-tauri/binaries`).
- Worktrees have no `frontend/node_modules`: `pnpm install --frozen-lockfile` first.

## Verify
```bash
cargo test -p deino-core --lib meetings::
cargo test -p deino --lib features::meetings
cargo clippy -p deino-core -p deino --all-targets --locked -- --deny warnings
cd frontend && pnpm typecheck && pnpm exec vitest run src/features/meetings-list && pnpm lint
```
