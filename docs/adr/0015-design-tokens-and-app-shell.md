# 0015. Use semantic design tokens, a light/dark/system theme and registry-driven shell

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-10

## Context

The inherited UI hard-coded about a thousand Tailwind palette classes
(`text-gray-600`, `bg-blue-600`, …) and shipped stock shadcn HSL variables
with an unused `.dark` block, so dark mode could not work and every screen
chose its own colours. PARITY_PLAN §3B asks for a calm, Granola-like UI in
two equally designed themes that follow macOS by default, and
`docs/references/ui.md` §2 specifies the exact tokens, type scale, radii,
shadows and motion. Later cards add sidebar destinations and meeting-page
tabs, so the shell has to grow without each card editing shared layout code.

## Decision

1. **Tokens.** `frontend/src/styles/tokens.css` defines every `ui.md` §2 token
   as a CSS variable for light (`:root`) and dark, and maps them to Tailwind v4
   utilities with `@theme inline` (`bg-bg-muted`, `text-fg-muted`,
   `border-border`, `bg-accent`, `shadow-popover`, `z-(--z-dialog)`, …).
   Components use only these names. ESLint rejects palette utilities in any
   string and hex values in class names. The type scale collapses to four
   sizes and two weights; inherited `text-lg`/`font-medium` alias onto it.
2. **Theme.** No `data-theme` on `<html>` means "System": the dark values sit
   under `@media (prefers-color-scheme: dark)`. An explicit choice sets
   `data-theme="light|dark"` (any element may carry it to render a subtree in
   one theme, which the picker previews and `/design` use). The choice is
   saved in the typed settings store (`ui.theme`, ADR 0013); the last
   applied theme is also cached in `localStorage` so a blocking
   `public/theme-boot.js` can apply it before the first paint.
   `features/appearance` puts a theme on screen and calls Tauri's
   `setTheme` so the title bar and traffic lights match;
   `features/settings` saves the choice and `SettingsSync` applies the
   saved value (also when it changes elsewhere). `tokens.test.ts`
   checks that both dark copies are identical and every text pair meets AA.
3. **Primitives.** shadcn/Radix components live in `frontend/src/ui/`,
   restyled to the tokens (button, input, select, switch, popover, menu,
   dialog, tooltip with shortcut, tabs, segmented control, chips, kbd,
   skeleton, empty state, page header, scroll area, toaster, level bars).
   Icons come only from `ui/icons.ts` (lucide, 16px, 1.5 stroke).
4. **Registries.** The sidebar renders `features/shell/nav.ts`, the ⌘K palette
   renders the providers in `features/shell/command-palette/providers.ts`,
   the meeting page renders `features/meeting/slots.ts`, and settings render
   `features/settings/sections.tsx`. A card adds an entry; it never edits the
   layout. Unbuilt destinations show a "coming soon" page from the registry.
5. **Shared UI state** (sidebar, right panel, palette, chat bar, title-bar
   slot) is one small Zustand store, `features/shell/layout-store.ts`.
6. **Window chrome.** The main window uses `titleBarStyle: "Overlay"`: the
   shell draws a 44px title bar with `data-tauri-drag-region` and pages
   portal their actions into it (`TitleBarActions`).
7. **Font.** Inter Variable, self-hosted with `@fontsource-variable/inter`
   (OFL-1.1), so the app makes no network request for fonts.

## Alternatives

- **`light-dark()` in one token block.** Shorter CSS, but needs Safari 17.5;
  Deino supports macOS 14.2 (Safari 17.2). Rejected; the duplicated dark block
  is guarded by a test instead.
- **`next-themes`.** Designed for class toggling and SSR; we need the Tauri
  window sync and an explicit "system" with no attribute. A 40-line store is
  simpler.
- **Keeping shadcn's variable names** (`--background`, `--primary`,
  `--accent` as a hover grey). Their `accent` collides with ours (the green
  "live / mine / focused" colour) and the names don't match `ui.md`. Only the
  BlockNote editor still reads them, through a scoped bridge
  (`styles/blocknote.css`) until NOT-01 replaces BlockNote.
- **React context for shell state.** Every consumer would re-render on any
  change; Zustand selectors avoid that and are what the plan names.

## Consequences

- Dark mode works everywhere that uses tokens; a new screen gets both themes
  for free, and `/design` (dev builds) shows every token and primitive in
  both themes side by side.
- The saved theme comes from the settings store, which loads over IPC; until
  it arrives the first-paint cache (`deino.theme` in `localStorage`) decides.
  If the two disagree (say the settings file was edited), the window shows the
  cached theme for a moment and then switches.
- Inherited screens were tokenised mechanically; their markup is still
  legacy and is rebuilt by the cards that own them (NOT-01, TRN-01, CAP-10,
  FND-11, ORG-02).
