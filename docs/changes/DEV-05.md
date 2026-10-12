# DEV-05 · Deep links, global hotkeys, Raycast, Shortcuts

## Progress log (PARITY_PLAN §7.1)
| DEV-05 | `deino://` links and system-wide hotkeys (ADR 2026-10-12-deep-links-and-hotkeys). `tauri-plugin-deep-link` =2.4.10 and `tauri-plugin-global-shortcut` =2.3.2 (pinned, MIT/Apache-2.0, `cargo deny` clean), registered from the feature's attach hooks; scheme in `tauri.conf.json`. Core: `deino_core::links` (`DeepLink::parse` validates `meeting/<id>[?t=]` + the `meetings/<id>` alias, `new-note[?event=]`, `record/start`, `record/stop`, `search?q=`, `recipe?data=`; `Intent`, `Navigation`, `LinkService` over `Recorder`/`LockProbe`/`Shell` ports with fakes) and `deino_core::hotkeys` (`Accelerator` validation, `HotkeyAction`, `HotkeyService` over `HotkeyRegistrar`, `FakeRegistrar`); `settings.shortcuts` (`ShortcutSettings`, defaults ⌃⌥⌘R / ⌃⌥⌘N / ⌃⌥⌘J). Plugins `links` (`take`; events `navigation-waiting`, `link-rejected`) and `hotkeys` (`list`, `set`), both `Guarded`. Recording links and the record hotkey go through `RecordingService::start`/`stop` (CAP-10's path); navigations wait in a queue the window pulls, which the app lock's guard refuses while locked. UI: `features/links` (link host in the shell's main layout, inside `LockGate`; `follow.ts` maps navigations to routes; recipe → "WIP" toast), `features/hotkeys` (Settings → Shortcuts: change by pressing keys, turn off, reset, inline conflict reasons). Exports now write `deino://meeting/<id>`. Docs: `docs/setup/raycast-and-shortcuts.md`. | |

## Follow-ups (PARITY_PLAN §7.2)
- Recipes (no card yet on main): `deino://recipe?data=` parses and validates but only shows "Recipes are not here yet". Route `Navigation::Recipe` to the Recipes import (and carry the data, which Rust keeps in `DeepLink::Recipe`) when Recipes ships.
- INT-00 / INT-01: `deino://oauth/callback` is refused as an unknown route; add it as a `DeepLink` variant (and `RedirectSpec`'s scheme variant) when a provider needs a custom-scheme redirect.
- Windows/Linux: enable `tauri-plugin-single-instance`'s `deep-link` feature so a second instance forwards its link (macOS needs nothing; a `lib.rs` change, so left out).
- AI-02 (D13): the Ask hotkey opens the meeting's right panel, whose first section is the Ask placeholder; if the user picked another section, the panel opens on that one. Focus the Ask input once AI-02 builds it.
- Settings patched by hand (or by `settings|patch`) with a new `shortcuts` value takes effect at the next launch; Settings → Shortcuts goes through `hotkeys|set`, which applies at once.
- Owner QA (manual, needs the bundled app): see below.

### Manual QA (Owner)
Build and install the app (`pnpm run tauri:build`, copy to `/Applications`), open it once, then:
1. **Background hotkeys:** with another app (Safari) in front and Deino in the background or minimised, press ⌃⌥⌘R: recording starts (record panel shows); press again: it stops. ⌃⌥⌘N brings Deino forward on a new note. ⌃⌥⌘J on a meeting page opens the right panel (Ask); elsewhere it just brings Deino forward.
2. **Hidden window:** close the main window (Deino keeps running in the menu bar) and repeat step 1.
3. **Change a hotkey:** Settings → Shortcuts → Change, press ⌃⌥K: the new one works in Safari, the old one no longer does. Pick ⌘⇧4: refused ("macOS uses that shortcut"). Pick another app's hotkey (e.g. Raycast's ⌥Space with ⌃): refused, the old one keeps working. Restart Deino: the change persists.
4. **Links:** in Terminal, `open "deino://search?q=standup"`, `open "deino://meeting/<id>?t=60000"` (from `deino meetings --json`), `open "deino://meetings/<id>"`, `open "deino://new-note"`, `open "deino://record/start"` then `.../record/stop`, `open "deino://recipe?data=x"` (toast "Recipes are not here yet"), `open "deino://search"` (toast "Deino could not open that link").
5. **Cold start:** quit Deino, `open "deino://meeting/<id>"`: Deino launches and opens the meeting.
6. **App lock:** turn on Settings → Privacy → App lock, lock (menu bar → Lock Deino), `open "deino://meeting/<id>"`: the lock screen comes forward and nothing shows; unlock: the meeting opens. While locked, `open "deino://record/start"` starts recording.
7. **Raycast / Shortcuts:** add one script and one shortcut from `docs/setup/raycast-and-shortcuts.md` and run them.
8. Screens: Settings → Shortcuts in light and dark.

## Architecture (ARCHITECTURE.md)
- Crates → `deino-core`: add `links` (DEV-05: `deino://` parsing, `Intent`/`Navigation`, `LinkService` holding navigations until the window takes them, never while locked) and `hotkeys` (DEV-05: accelerator validation, `HotkeyService` over a registrar port).
- App crate: plugins `links` and `hotkeys`; `composition/links.rs` registers `tauri-plugin-deep-link` and `composition/hotkeys.rs` `tauri-plugin-global-shortcut` from `Phase::Startup` hooks (orders 70, 71). The scheme lives in `tauri.conf.json`.
- Frontend: `features/links` (the link host, mounted by `shell/hooks/use-links.ts` inside `LockGate`), `features/hotkeys` (Settings → Shortcuts).

## CLAUDE.md
- Crate table, `deino-core` row: add "`links` and `hotkeys` (DEV-05: `deino://` links parsed and validated in Rust, held behind the app lock; system-wide hotkeys; ADR 2026-10-12-deep-links-and-hotkeys)".
- Patterns worth knowing: "**Links:** a new `deino://` route is a `DeepLink` variant plus its parse arm and tests (`deino_core::links`); what it shows is a `Navigation` the UI maps in `features/links/follow.ts`. Meeting links are `deino://meeting/<id>[?t=<ms>]`."

## Plan updates
- DEV-05: tick "Every link route is tested" (`links/parse_tests.rs`, `links/service_tests.rs`, `features/links/contract_tests.rs`, `features/links/follow.test.ts`). "Hotkeys work while the app is in the background" is on the manual QA list above (registration and conflicts are tested with `FakeRegistrar`).
- §7.2: closes the EXP-01, DEV-02 and TRN-04 rows about `deino://meetings/` vs `deino://meeting/` (both open; exports write `meeting/`). The INT-00 row's "custom-scheme redirect needs DEV-05's URL handler" is now unblocked (see Follow-ups).
