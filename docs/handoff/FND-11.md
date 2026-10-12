# FND-11 Onboarding v2: handoff A to B (`feat/FND-11-onboarding`; phase A done)

## Done (phase A)
- **Progress model:** `Settings.onboarding` is now `{ completed, step: OnboardingStep, outcomes: { [step]: done|skipped } }`
  (`crates/deino-core/src/settings/onboarding.rs`). Older files keep only `completed`; the legacy import maps
  `onboarding-status.json` to `OnboardingSettings::finished()`.
- **Step machine:** `frontend/src/features/onboarding/machine.ts` (pure `transition`; continue, skip, back, open, finish,
  resume). The UI saves each move as a settings patch, with an optimistic cache update (`queries.ts`).
- **Permissions:** `deino-core::permissions` (model, `PermissionGate` port, `PermissionService`, fake). The macOS adapter is
  `src-tauri/src/permissions{.rs,/macos.rs,/other.rs}` (AVFoundation mic, the legacy tap probe for system audio,
  AX trust, the notification plugin, System Settings deep links). The typed plugin `onboarding` exposes `permissions`,
  `request_permission` and `open_permission_settings`. Calendar access reuses the calendar plugin.
- **Steps (UI):** welcome, permissions (polled every 1.5 s), AI setup (Parakeet and the embedding model auto-download with
  progress; a Claude or OpenAI key is checked with `llm.models(provider, key)` and then saved, and OpenAI also pins every
  generating route; "On this Mac" reuses the legacy `BuiltInModelManager`), calendar (`CalendarSettings`), profile
  (`ProfilePage`), and try it (a 20 s test through `recording.start/stop`, then "Open it" goes to `/meetings/$meetingId`).
- **Gate:** `useOnboardingGate()` returns `pending | onboarding | app`, and the shell renders `OnboardingFlow` lazily inside
  `Suspense`. Settings → Setup (`ResumeSetup`) lists unfinished steps and dispatches `resume`.
- **First launch:** `database/setup.rs` now creates the fresh database at startup. The old onboarding created it from the UI.
- **Deleted:** legacy `components/onboarding/*`, `contexts/OnboardingContext.tsx`, `types/onboarding.ts`, app `onboarding.rs`
  (4 legacy commands), `transcription/default-model.ts`, the dead `onDownloadProgress`, and `resolveOnboardingSummaryModelStatus`.
- **dev:mock:** `?onboarding` starts a fresh flow (`settings-store.ts`), with `mock-ipc/onboarding.ts`. This also fixed a
  latent mock import cycle: `mock-database.ts` no longer imports the meetings feature at runtime.

## Left for B
1. Playwright e2e over `pnpm dev:mock` (`/?onboarding`): walk all six steps, skip-all, resume from Settings → Setup.
2. Visual polish against `docs/references/ui.md` in light and dark, with screenshots. The welcome step is plain
   (no mark or illustration), and the AI step is long. Consider collapsing the built-in model manager, which is a legacy
   component with its own styling.
3. The profile step reuses `ProfilePage`, which has its own Save button. Continue does not save unsaved edits, so make
   Continue submit or warn.
4. **Sample meeting (card step 6):** not built. The only sample data is `deino-testkit` fixtures, which are test-support
   and dev-only. A release-safe importer (a bundled fixture as a resource, imported through the repos with no fake
   calendar account) is real work. Either build it or record it as a §7.2 follow-up with the owner's OK.
5. Docs: ARCHITECTURE.md (the `onboarding` plugin, `deino-core::permissions`, `src-tauri/src/permissions`, DB created at
   startup), PARITY_PLAN §7.2 rows for FND-11 (mark FND-04/06/10a/CAP-01/CAP-03/AI-01 follow-ups done or carried over),
   CLAUDE.md plugin list, an ADR if wanted (permissions port). Also check the card's "<3 min to first recording".
6. Full gate: `scripts/ci-local.sh`. Not run here; only targeted checks were.
7. Manual QA on a real Mac with a fresh data dir: each prompt (mic, the system audio probe, AX prompt, notifications),
   the deep links, and the first recording.

## Gotchas
- macOS cannot read the system-audio or notification permission without asking. The service remembers the last request's
  answer for system audio, and the notification plugin reports `granted` on desktop.
- The mic request uses `cidre::blocks::comp1` directly, because cidre's async helper holds a non-`Send` ref across `.await`.
- Permissions rows auto-refresh (refetchInterval 1500 ms plus focus). Turn this off if it is noisy in e2e.
- The onboarding index must stay light (no step imports). The steps load lazily to avoid module cycles through the mocks.
- Built-in summary model: `initialize_fresh_database` still defaults `ai.provider` to builtin when `ai.model` is empty.
  The onboarding Claude/OpenAI choice overwrites it.

## Verify
```bash
export RUSTC_WRAPPER=sccache CARGO_BUILD_JOBS=6
cargo test -p deino-core --lib -- settings permissions
cargo test -p deino --lib -- onboarding permissions bindings features::
cargo clippy -p deino-core -p deino --all-targets --locked -- --deny warnings
cd frontend && pnpm typecheck && npx eslint src/features/onboarding src/features/shell src/dev/mock-ipc
npx vitest run src/features/onboarding src/features/settings src/features/shell src/dev
npx vite --mode mock --port 3187 --strictPort   # then open http://localhost:3187/?onboarding
```
