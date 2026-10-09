# Testing

How Deino is tested, what is worth a test, and the command for each check.
PARITY_PLAN §3A ("Testing") is the rule; this file is the how-to. The
philosophy is adapted from anarlog's testing guide (MIT).

## Commands

Run from the repository root unless noted. `scripts/ci-local.sh` runs all of
them in order, fails fast and prints timings; GitHub CI calls the same steps by
name, so a green local run is a green CI run. Each row is also one command.

| Check | Command |
|---|---|
| Everything (the merge gate) | `scripts/ci-local.sh` (`--list` for steps and groups) |
| Rust tests | `cargo nextest run --workspace` (else `cargo test --workspace`), then `cargo test --doc --workspace` |
| Dependency rules | `scripts/check-dependency-rules.sh` |
| Rust lints and spelling | `scripts/clippy.sh` |
| Rust format | `cargo fmt --all --check` |
| Rust file size | `scripts/check-rust-file-size.sh` |
| sqlx offline data | `scripts/sqlx-prepare.sh --check` (needs sqlx-cli 0.8) |
| Licences, bans, sources | `cargo deny check licenses bans sources` (advisories warn; ignores in `deny.toml`) |
| TypeScript bindings | `cargo test -p deino --lib bindings`, then `git diff --exit-code frontend/src/lib/ipc/generated` |
| Frontend tests | `cd frontend && pnpm test` |
| Frontend types | `cd frontend && pnpm typecheck` |
| Frontend lint | `cd frontend && pnpm lint` |
| Frontend format | `cd frontend && pnpm format:check` |
| Frontend build | `cd frontend && pnpm build` |
| UI smoke tests | `cd frontend && pnpm test:e2e` (Playwright in the installed Chrome over `pnpm dev:mock`; screenshots in `frontend/test-results/`) |
| Recording acceptance (macOS, real time) | `cargo test -p deino-audio --test kill_harness -- --ignored --nocapture --test-threads=1` (`kill -9` loss and CPU per track) |
| UI by hand | `cd frontend && pnpm dev:mock`, then open http://localhost:3119 |

Every test must pass on every machine. A test never reads real hardware:
inject the platform's answer instead (`InputDeviceKind::classify` takes the
native detection result as an argument, so its tests pass `None`).

## Where CI runs what

Pushes run on Linux (macOS minutes cost 10x on the private repo):

| Job | Steps |
|---|---|
| `rust (portable crates)` | `fmt file-size deps deny clippy-portable test-portable`: clippy and tests for every crate except the app and the sidecar |
| `rust (app on Linux)` | `check bindings test-app sqlx`: the workspace compiles with macOS code cfg'd out, bindings and `.sqlx/` are fresh, the app's unit tests pass |
| `frontend` | `frontend`: install, typecheck, lint, format, Vitest, Vite build, Playwright smoke tests (screenshots uploaded as the `e2e-screenshots` artifact) |
| `pr title` | Conventional Commits, on pull requests |

The macOS-only code (Core Audio, the app's clippy and tests on macOS) is
covered by `scripts/ci-local.sh` before every merge, and by the manual (or,
with a self-hosted runner, weekly) `macos` job.

## Write a test when

- **A bug is fixed.** Start with one test that fails without the fix.
- **A contract is observable:** a command's JSON in and out, a typed event, a
  migration, a parser or serializer, the generated bindings.
- **Losing it loses data or trust:** migrations, transactions, persistence,
  privacy (nothing sensitive in logs), recording and transcription lifecycles.
- **An invariant spans time or tasks:** ordering, retries, cancellation,
  recovery after a restart.

## Do not write a test for

- CSS classes, DOM structure, copy or log text that is not a contract.
- Mock call counts when the observable result already proves the behaviour.
- Private helpers already covered through their public caller.
- Framework behaviour (React, TanStack, Tauri, Tokio, serde).
- File layout or import boundaries: those are lint rules.

Every test's name says the regression it prevents
(`rename_rejects_a_blank_title_before_touching_storage`, not `test_rename_2`).

## Where tests go

| Layer | Test | Example |
|---|---|---|
| Model, service | Unit tests with fakes (`FakeMeetingRepo`, `FakeClock`), no I/O | `crates/deino-core/src/meetings/service.rs` |
| Repository | Against `deino_db::test_db()` (in-memory, every migration, foreign keys on); list queries assert their `EXPLAIN QUERY PLAN` (`testing::query_plan`) | `crates/deino-core/src/meetings/repo.rs` |
| Port contract | One `async fn` per behaviour, generic over the port, called once with the SQLite repository and once with the fake, so the fake cannot drift | `crates/deino-core/src/segments/repo_tests.rs` |
| Schema, pools | Migrations, persistence across reopen, commit signals | `crates/deino-db/src/database_tests.rs` |
| Migration round trip | Apply the migrations before a version, seed rows in the old format, run the rest, assert (anarlog's technique) | `crates/deino-db/src/migration_tests.rs` |
| Command | Contract test through the real plugin and ACL on Tauri's mock runtime (`features::ipc_tests`) | `frontend/src-tauri/src/features/meetings/contract_tests.rs` |
| Bindings | Export, manifest drift, ID branding | `frontend/src-tauri/src/bindings.rs` |
| Hook | Vitest + Testing Library with the generated bindings mocked and typed (`satisfies typeof Generated.commands`) | `frontend/src/features/meetings/queries.test.tsx` |
| Component | Vitest, render with a `QueryClient`, interact through roles and labels | `frontend/src/features/_example/components/example-notes.test.tsx` |
| Main flow | Playwright over `pnpm dev:mock`, fixture data, one spec per flow | `frontend/e2e/meeting.spec.ts` |

Conventions:

- Rust unit tests live in a `#[cfg(test)] mod tests` at the end of the file.
  When they pass about 150 lines, move them to a sibling `<module>_tests.rs`
  (the 300-line cap excludes tests).
- Fakes live next to the trait they implement, behind the crate's
  `test-support` feature, and honour the trait's whole contract (ordering
  included). No mocking frameworks.
- Time comes from `Clock`; never sleep in a test. Waiting for an event uses a
  channel with a timeout constant.
- Seed rows that no service can create yet with `deino_testkit::seed`, and
  whole realistic meetings with `deino_testkit::seed_fixtures` (the eight
  meetings in `fixtures/`, see its README). AI features evaluate against each
  fixture's `groundTruth`.
- `pnpm dev:mock` swaps every generated binding module for its mock in
  `frontend/src/dev/mock-ipc/`. A card that adds or changes a command updates
  that mock in the same PR (typecheck and `mock-ipc.test.ts` fail until it
  does); never grow the legacy shim (`legacy*.ts`).
- Frontend tests are colocated `*.test.ts(x)`. Mock the generated module
  (`vi.mock('@/lib/ipc/generated/<feature>')`), never `invoke` strings, so a
  Rust signature change breaks the mock at compile time.
- Rendering anything under `SidebarProvider` needs a `QueryClientProvider`.

## Coverage

New service and repository code should reach about 80% line coverage
(`cargo llvm-cov`, `vitest --coverage`), measured on demand; CI does not
report it yet (PARITY_PLAN §7.2).
