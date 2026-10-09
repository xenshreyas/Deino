# 0002. Register IPC as one inline Tauri plugin per feature

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-02 (decided in PARITY_PLAN §4.0 from `docs/references/architecture.md` §1)

## Context

Deino inherited about 190 Tauri commands registered in one
`tauri::generate_handler![]` inside an 855-line `lib.rs`. Every card had to
edit that list, so parallel cards collided on the same lines, and the command
names (`api_get_meetings`, `builtin_ai_list_models`) carried their feature as
a prefix because there was no namespace.

Facts that shape the choice (verified in the Tauri 2.11 sources):

- Plugin commands (`plugin:<name>|<cmd>`) are always checked against the
  capability ACL. App commands are only checked once an `AppManifest` exists.
- `tauri-build` supports plugins that live inside the app crate:
  `Attributes::plugin(name, InlinedPlugin::new().commands(..).default_permission(AllowAllCommands))`
  generates `allow-<cmd>`/`deny-<cmd>` permissions and a `<name>:default` set,
  with no separate crate or permission files.
- `tauri_specta::Builder::commands()` overwrites earlier calls, so one global
  specta builder cannot be composed from modules; one builder per plugin can.

## Decision

New IPC is one inline Tauri plugin per feature in the app crate:

- `src/features/<feature>.rs` (+ `features/<feature>/{commands,events}.rs`)
  exposes `pub const NAME` and `pub fn init() -> TauriPlugin<R>`. Commands are
  thin and call into the Tauri-free core.
- A feature is registered with three one-line, append-only edits:
  1. `src/features/manifest.rs`: `("<feature>", &["cmd_a", "cmd_b"])`. The file
     is `include!`d by `build.rs`, which turns every entry into an
     `InlinedPlugin` with `AllowAllCommands`.
  2. `src/lib.rs`: `.plugin(features::<feature>::init())`.
  3. `capabilities/default.json`: `"<feature>:default"`.
- Capabilities live in `capabilities/default.json`, not `tauri.conf.json`.
- The UI calls `invoke('plugin:<feature>|<cmd>')` (through generated
  tauri-specta bindings once FND-13 adds them).
- Inherited commands stay app commands in `src/legacy.rs` under one
  `generate_handler!` until a card ports them. No `AppManifest` is added,
  because it would put every legacy command under the ACL at once.
- Guard tests: `features.rs` asserts every manifest feature is granted in
  `capabilities/default.json`, and `features/ipc_tests.rs` calls a plugin
  command through the real generated ACL on Tauri's mock runtime (granted for
  the `main` window, denied for any other window).

The first plugin is `system` with one command, `app_version`.

## Alternatives

- **Keep one global `generate_handler!`:** simplest, but every card edits the
  same list, there is no namespace and no ACL on new commands.
- **One plugin crate per feature (anarlog `plugins/*`):** strong isolation, but
  each plugin needs its own Cargo.toml, permissions directory and JS package.
  Too much boilerplate for a single app.
- **`inventory`-based self-registration (spacedrive):** no central list, but it
  hides what is registered and depends on linker behaviour; the ACL still needs
  an explicit list in `build.rs`.
- **Add an `AppManifest` and ACL the legacy commands:** right long-term, but it
  would require listing and granting all ~150 legacy commands in one change.

## Consequences

- Parallel cards touch different lines; registration conflicts become trivial.
- New commands are short and namespaced (`plugin:meetings|list`) and are
  ACL-checked from day one.
- Forgetting the capability line fails the guard test in CI, not at runtime.
- Command names exist twice (the manifest and `generate_handler!`) until
  FND-13's tauri-specta bindings add a manifest-drift test.
- The legacy surface shrinks card by card; when it is empty, an `AppManifest`
  can be added to ACL the remaining app commands.
