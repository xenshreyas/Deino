# Deep links and hotkeys: parsed in Rust, pulled by the window behind the lock

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** DEV-05

## Context

DEV-05 adds `deino://` links (from exports, the CLI, MCP, transcript moments,
Raycast and Shortcuts) and system-wide hotkeys. Three questions needed an
answer:

1. **Where links are understood.** `tauri-plugin-deep-link` can hand URLs to
   JavaScript, but a URL is untrusted input from any app on the Mac.
2. **The app lock (PRV-02).** A link may arrive while Deino is locked. It must
   not show a meeting, a search or a note until the owner unlocks, and
   recording links should keep working, as recording does behind the lock.
3. **Registration.** Both plugins are third-party Tauri plugins, but `lib.rs`
   may not grow per-feature lines (ADR 2026-10-12-feature-self-registration).

## Decision

- **Rust parses every link** into `deino_core::links::DeepLink`, validating
  the route and each parameter (meeting IDs, `t`, calendar event UUIDs, a
  non-empty query, recipe data). `deino://meetings/<id>`, the shape EXP-01's
  exports wrote, is an alias; exports now write `deino://meeting/<id>`.
- **Links and hotkeys share one path.** Both become an `Intent`. Recording
  intents call the recorder at once (CAP-10's `RecordingService` path).
  Anything that shows the library becomes a typed `Navigation` in a short
  queue (8) that the window **pulls** with `plugin:links|take`.
- **The lock is enforced by the pull.** `links` is a `Guarded` plugin, so the
  IPC guard refuses `take` while locked, and the service hands out nothing
  then either. The window's link host lives inside `LockGate`, which mounts
  it right after an unlock, and takes on mount. The push event
  (`navigation-waiting`) carries nothing.
- **The plugins register from the feature's attach hooks**
  (`composition/links.rs`, `composition/hotkeys.rs`, `Phase::Startup`) with
  `AppHandle::plugin`, after the app's own plugins. The scheme is declared in
  `tauri.conf.json` (`plugins.deep-link.desktop.schemes`).
- **Hotkeys are validated in `deino_core::hotkeys`** and registered before
  they are saved, so a combination another app holds is refused, not saved
  dead. A system-wide hotkey needs two modifiers, one of them ⌘, ⌃ or ⌥
  (F1–F20 need none), and macOS's own shortcuts are refused.

## Consequences

- A link never reaches the webview as text; the UI only maps typed
  navigations onto routes (`features/links/follow.ts`).
- The queue is in memory: links that arrive while locked survive until the
  unlock, not a quit.
- A launching link reaches the app after setup, so registering the plugin
  from a startup hook misses nothing on macOS. Windows and Linux would need
  `tauri-plugin-single-instance`'s `deep-link` feature to forward a second
  instance's link (a follow-up; macOS is the target).
- Recording links work while locked, like the menu bar.
