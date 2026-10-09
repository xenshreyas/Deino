# Deino UI reference: design system, components, screens

Research input for **FND-10** (and NOT-01, NOT-02, TRN-01, CAP-10, ORG-02, AI-02). Read-only study, 2026-10-08.

**Sources**
- **Live products, measured in the browser with computed styles:**
  - granola.ai, including its public `oats` CSS token set (light and dark) and its changelog app screenshots
  - circleback.ai, including its live HTML app demo at `circleback.ai/embed/demo` and its full light/dark `--color-*` token set
  - gumloop.com
  - linear.app
- **Reference repos** in `~/Documents/GitHub/deino-references/`: anarlog, Handy, openwhispr, muesli, spacedrive and Cap.
- **Deino's current frontend.**

Screenshots are in `ui-shots/` (see §8).

---

## 0. Findings in one page

### What the benchmarks actually do (measured)

| | Granola | Circleback | Gumloop | Linear |
|---|---|---|---|---|
| **Neutrals** | **Warm** "oats": ink `#292929`, secondary `#72726E`, tertiary `#ACADA8`, surface `#F7F7F2`, sunken `#F2F2EC`, hairline `#47432A33` | **Pure neutral** greys: label `rgb(39,39,39)`, secondary `127`, separator `239`, bg-secondary/navigation `250` | **Cool**: ink lab(8.7) ≈ `#16171A`, muted `#5F6068`/0.88α, border lab(93) ≈ `#E8E8EC`, bg `#FAFAFA` | Cool dark: bg `#08090A`, text `#D0D6E0`, muted `#8A8F98`, labels `#62666D` |
| **Accent** | Olive/lime: fill `#5B6F00`, ink `#788C15`; recording bars `#74BD00`; danger `#E95D3D` | Blue `#4262FF` (UI), brand orange `#F24E1D`, record red `#FF393C` | None in the app; black primary | Indigo (sparingly) |
| **Dark mode** | `html.dark`: surface `#292929`, raised `#313130`, elevated `#363635`, sunken `#212121`, ink `#FCFCF8`, secondary `#9E9E99`, hairline `#FFFFFF26` (warm, not black) | `.dark`: bg `rgb(29,29,29)`, secondary `35`, tertiary `42`, elevated `49`, label `202`, secondary label `144`, separator `49` | (light only on site) | Near-black, low-contrast borders |
| **Font** | App: system/SF-like sans ("melange" on web); serif only for marketing | `-apple-system` system stack | Geist Sans (UI), Gellix (marketing) | Inter Variable |
| **UI text** | 13–14px | Nav 13/500, body 15/450 at 28px line-height, section heads 15/600, meta 12/450 | 14/500 items, 12/500 section labels | Nav 13/510, body 14/21, meta 12/510, title 20/590 at −0.24px tracking |
| **Nav row** | ~30px pill, soft fill on active | 31px, r7, sidebar 204px + 12px gutters ≈ 228px | 23–28px, r8 | 28px, r8, sidebar ≈ 224px |
| **Cards** | Rare; the note is the page | r12, 1px `#EFEFEF` border, shadow `0 0 0 .5px #00000016, 0 3px 6px -2px #00000005, 0 1px 1px #0000000a` | r8, 1px border, no shadow | Hairline, no shadow |
| **AI vs user** | User text in ink, AI text in grey; section headers small and bold | Uniform (all AI) | — | — |
| **Transcript** | **Bubbles**: "Me" right-aligned, solid teal, white text, r≈6, 2–4px stacking; hover shows `00:00` + copy + delete | No bubbles: avatar + name, timestamp right, paragraph below | — | — |
| **Recording** | "Transcribing" pill bottom-centre of the note with dancing bars; same slot becomes the "Ask anything" bar | Red record colour; minimal floating panel | — | — |
| **Chat bar** | Bottom-centre pill "Ask anything" + recipe chips; agentic steps in grey with icons; user turns are right-aligned grey bubbles, AI is plain text | Bottom "Ask about <meeting>…" with @, attach and a circular send | User bubble with avatar, steps list, tables | Side panel "Reply…" |

### Deino baseline (what FND-10 replaces)

- **Theme tokens:** `frontend/src/app/globals.css` holds stock shadcn "neutral" HSL tokens. The `.dark` block exists but is effectively unused: only 2 component files use `dark:`.
- **Hard-coded colours:** about **1,069 raw palette classes** across `components/` and `app/` (`text-gray-600` ×96, `border-gray-200` ×57, `bg-blue-600` ×30, and so on). Dark mode cannot work until these are replaced with semantic tokens.
- **Font:** Source Sans 3 via `next/font/google` in `app/layout.tsx`.
- **Sidebar:** `components/Sidebar/index.tsx` is 882 lines of 20px icons with hard-coded colours.
- **Dependencies already present that we keep:**
  - BlockNote 0.36 (`core` / `react` / `shadcn`)
  - cmdk 1.1
  - sonner 2
  - framer-motion 11
  - lucide-react
  - `@tanstack/react-virtual`
  - Radix
  - tailwindcss-animate
- **Stray dependencies:** `@tiptap/*` 2.10 is listed as well. Remove it if nothing imports it directly; BlockNote brings its own TipTap.

---

## 1. Design principles (10 rules)

1. **The note is the page.** Meeting content renders as a document: a 720px column (`max-w-[45rem]`), 16px text at 26px line-height. There is no card chrome around notes, transcript or chat; we never use cards inside cards. This follows Granola, and muesli's 720pt reading column.
2. **One accent, used for state.** The neutrals are warm greys. The single accent is a calm green (`--accent`), and it means *live / mine / focused*:
   - recording level bars
   - "Me" transcript bubbles
   - the focus ring
   - text selection
   - the active toggle

   Primary buttons are **ink** (`--fg` fill), not accent, as in Granola, Gumloop and Linear. Red (`--recording`) appears only as the record dot.
3. **Hierarchy comes from type colour, not boxes.** Four sizes (12/14/16/24) and two weights (400/600). Secondary information steps down to `--fg-muted`, never to a smaller size than 12px.
   - AI-written text is `--ai-text` (grey).
   - User-typed text is `--user-text` (ink).

   The difference reads at a glance (≈3:1 between them), and each still passes AA against the background.
4. **Hairlines, not borders.** 1px `--border` separators. Strong borders appear only on inputs (on hover or focus) and on segmented controls. No gradients. Shadows go only on layers that float: popovers, menus, dialogs, toasts, the chat bar and the record pill.
5. **Calm density.** Nav rows are 30px. List rows are 40px for one line and 56px for two. Every row inside a group of items sits on the 4px grid.
6. **Content-first chrome.**
   - A slim, resizable sidebar: 232px by default, 200–320px, collapsible with `[`.
   - A 44px title bar that is a drag region with traffic lights inset.
   - Page titles live *in* the content (24/600), not in a toolbar.
   - The right panel (chat, participants, details) is optional and toggled with `]`.
7. **Keyboard-first and discoverable.** Shortcuts:
   - ⌘K palette
   - ⌘J chat
   - ⌘N new note
   - ⌘⇧R record
   - `[` and `]` panels
   - ⌘F search transcript

   Every icon-only button has a tooltip that shows its shortcut in a `Kbd`.
8. **Quiet motion.**
   - 120ms for hover and colour, 160ms for menus and popovers, 200ms for dialogs and panels. All use one ease-out curve, `cubic-bezier(0.22,1,0.36,1)` (the shared house curve of anarlog and Handy).
   - Nothing bounces or overshoots. The record-pill morph is the only longer animation (240ms).
   - Under `prefers-reduced-motion`, everything except an 80ms opacity fade is removed.
9. **Every surface has a resting state.**
   - Each list has an empty state with one action and its shortcut.
   - Loading shows skeletons with the shape of the content. The only spinners are inside buttons, and only after 400ms.
   - Errors are inline, written in plain English, and include a fix action.
10. **Both themes are designed, and the app follows the system.**
    - Dark mode is a warm charcoal (`#1A1A18`), not black, with raised layers getting *lighter*.
    - The sidebar is the darkest plane in dark mode and the lightest tint in light mode.
    - Shadows become hairline rings plus deep soft blur in dark mode.
    - Theme options are System, Light and Dark; the default is System. The theme is applied before first paint so there is no flash, and it is synced to every Tauri window, including the record panel.

---

## 2. Token spec (FND-10 implements these as CSS variables)

### 2.1 Implementation notes

- **Format:** store each colour as space-separated RGB channels (Circleback's pattern) so Tailwind 3.4 opacity modifiers work:
  - `--fg: 31 31 28;`
  - `colors.fg = 'rgb(var(--fg) / <alpha-value>)'`

  The hex values below are the source of truth.
- **Where and how:**
  - Put the tokens in `globals.css` under `:root` and `.dark`.
  - `darkMode: ['class']` is already configured.
  - Expose the tokens as Tailwind colours named exactly as the tokens (`bg`, `bg-subtle`, `fg-muted`, …).
- **Keep shadcn working** by aliasing its variables to ours:

  | shadcn | Deino token |
  |---|---|
  | `--background` | `bg` |
  | `--foreground` | `fg` |
  | `--card` | `surface` |
  | `--popover` | `surface-raised` |
  | `--primary` | `fg` |
  | `--primary-foreground` | `bg` |
  | `--secondary` | `bg-muted` |
  | `--muted` | `bg-muted` |
  | `--muted-foreground` | `fg-muted` |
  | `--accent` (hover fill) | `bg-muted` |
  | `--destructive` | `danger` |
  | `--border` / `--input` | `border` |
  | `--ring` | `focus-ring` |

  Note that shadcn's "accent" means a hover grey, unlike our accent.
- **Lint for it:** add a rule in FND-03 that forbids raw palette classes (`(bg|text|border|ring)-(gray|slate|zinc|neutral|stone|blue|red|green|yellow)-\d+`) and hex values in TSX.

### 2.2 Colour tokens

Neutrals are warm (hue about 60, saturation 2–4%), sitting between Granola's oats and Circleback's pure greys. The accent is a calm forest green.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#FBFBF9` | `#1A1A18` | App canvas, note page |
| `--bg-subtle` | `#F5F5F2` | `#151513` | Sidebar, settings nav, title bar |
| `--bg-muted` | `#EDEDE9` | `#252522` | Hover fills, active nav, segmented track, "them" bubble, skeleton |
| `--surface` | `#FFFFFF` | `#20201E` | Inputs, cards (rare), chat bar |
| `--surface-raised` | `#FFFFFF` | `#2A2A27` | Popover, menu, dialog, toast, palette, record pill |
| `--fg` | `#1F1F1C` | `#EDEDE9` | Primary text, icons on hover, primary button fill |
| `--fg-muted` | `#65655F` | `#A6A69F` | Secondary text, meta, idle nav, idle icons |
| `--fg-subtle` | `#8C8C85` | `#77776F` | Placeholder, disabled, kbd hints, timestamps ≥12px decorative. **Not for essential text** (3.3:1 light, 3.9:1 dark) |
| `--border` | `#E6E6E1` | `#2E2E2A` | Hairlines, dividers, card and input rest border |
| `--border-strong` | `#D3D3CC` | `#3E3E39` | Input hover, segmented control outline, kbd |
| `--accent` | `#2F6E4E` | `#7CC39A` | Live and recording bars, focus, toggles on, links, Me-bubble edge, selected checkbox |
| `--accent-fg` | `#FFFFFF` | `#0E1A13` | Text and icons on solid accent |
| `--accent-soft` | `#E4EFE8` | `#20352A` | Me bubble, selected-row tint, accent badges |
| `--recording` | `#E5484D` | `#F2555A` | The record dot and stop-button glyph only (non-text) |
| `--danger` | `#C4362C` | `#F2786C` | Destructive text, buttons and error text |
| `--danger-soft` | `#FBE9E7` | `#3A1F1C` | Inline error background |
| `--warning` | `#B25A00` | `#F0A04B` | Recording-health warning (orange bars and text) |
| `--warning-soft` | `#FBEEDC` | `#3A2A17` | Warning banner background (text `#8A4500` light) |
| `--success` | `#2B7A4B` | `#6BCB8F` | Saved, synced, connected |
| `--ai-text` | `#6B6B65` | `#9E9E97` | AI-origin note text (NOT-02), AI chat "thinking" steps |
| `--user-text` | `#1F1F1C` | `#EDEDE9` | User-origin note text (= `fg`) |
| `--focus-ring` | `#2F6E4E` | `#7CC39A` | 2px outline, 2px offset (solid, so it passes 3:1 non-text contrast) |
| `--selection` | `#D4E6DA` | `#2B4A38` | `::selection`, search-hit current |
| `--highlight` | `#FFF3B0` | `#4D4413` | Search hits, current transcript sentence (Circleback uses 255 248 197 / 82 79 0) |
| `--overlay` | `rgba(20,20,18,.32)` | `rgba(0,0,0,.60)` | Dialog and palette scrim |

**Speaker palette** (`--speaker-1…6`), used for transcript names, chips and avatars. All pass AA as text on `--bg`:

| Speaker | Light | Dark |
|---|---|---|
| 1 | `#2F5FB3` | `#8FB2F0` |
| 2 | `#7048B8` | `#B9A0EE` |
| 3 | `#1B6F6F` | `#6CC6C2` |
| 4 | `#A8551A` | `#E9A06A` |
| 5 | `#B03A75` | `#E98FBE` |
| 6 | `#6B6A1F` | `#C4C26A` |

Assign colours deterministically by speaker id. anarlog's golden-angle OKLCH approach (`utils.ts`) is an alternative if there are more than 6 speakers.

### 2.3 Contrast check (WCAG 2.x, computed)

**Light:**

| Pair | Ratio |
|---|---|
| fg / bg | **15.95** |
| fg / bg-subtle | 15.13 |
| fg / bg-muted | 14.08 |
| fg / surface | 16.52 |
| fg-muted / bg | **5.66** |
| fg-muted / bg-subtle | 5.37 |
| fg-muted / bg-muted | 5.00 |
| fg-muted / surface | 5.87 |
| ai-text / bg | **5.18** |
| ai-text / surface | 5.36 |
| user-text / bg | 15.95 |
| accent-fg / accent | **6.07** |
| accent / bg (as link text) | 5.86 |
| fg / accent-soft | 14.01 |
| danger / bg | 5.18 |
| warning / bg | 4.65 |
| success / bg | 5.08 |
| fg / selection | 12.69 |
| warning-text / warning-soft | 6.26 |
| white / danger (filled button) | 5.37 |
| fg-subtle / bg | 3.27 (placeholder and decorative only; ≥3:1 non-text) |
| recording / bg | 3.78 (non-text dot; ≥3:1) |
| ai-text vs user-text | 3.08 (distinguishable) |
| Speakers / bg | 4.92–6.13 |

**Dark:**

| Pair | Ratio |
|---|---|
| fg / bg | **14.85** |
| fg / bg-subtle | 15.58 |
| fg / bg-muted | 13.09 |
| fg / surface | 13.90 |
| fg / surface-raised | 12.26 |
| fg-muted / bg | **7.12** |
| fg-muted / bg-muted | 6.28 |
| fg-muted / surface | 6.67 |
| ai-text / bg | **6.47** |
| ai-text / surface | 6.06 |
| accent-fg / accent | **8.61** |
| accent / bg | 8.41 |
| fg / accent-soft | 11.15 |
| danger / bg | 6.37 |
| warning / bg | 8.16 |
| success / bg | 8.77 |
| fg / selection | 8.36 |
| dark text / danger fill | 6.94 |
| fg-subtle / bg | 3.86 (decorative only) |
| recording / bg | 5.16 |
| ai-text vs user-text | 2.30 |
| Speakers / bg | 7.58–9.36 |

All text pairs pass **AA (≥4.5:1)** in both themes. `fg-subtle` and `recording` are only for non-text or placeholder use and meet the 3:1 non-text minimum.

The contrast script is at `scratchpad/tools/contrast.py`.

### 2.4 Typography

- **Font:**
  - `--font-sans: "Inter Variable", -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif`
  - `--font-mono: ui-monospace, "SF Mono", Menlo, monospace` (timestamps can stay sans with `tabular-nums`)
- **Font loading:** self-host with `@fontsource-variable/inter` (OFL); import it once in `layout.tsx` and delete `Source_Sans_3`.
  - This keeps no network at build time or runtime, which matters for a private app.
  - Set `font-feature-settings: "cv11", "ss03"` (single-storey a off, rounded quotes) and `-webkit-font-smoothing: antialiased`.
  - **Alternative:** if the bundle must be minimal, use the system stack only (anarlog does this) and the scale below still works.

**Scale** (4 sizes):

| Token | Size / line-height | Tracking | Use |
|---|---|---|---|
| `text-xs` | 12 / 16 | 0 | Meta, timestamps, labels, kbd, chips, section labels in the sidebar |
| `text-sm` | 14 / 20 | −0.005em | UI default: nav, buttons, inputs, list rows, transcript bubbles, chat |
| `text-base` | 16 / 26 | −0.011em | Note body (editor), chat answers in the full-page view, dialog titles (600) |
| `text-xl` | 24 / 32 | −0.019em | Page title, meeting title |

- **Weights:** `400` regular and `600` semibold, nothing else.
  - Note headings inside the editor use base/600 (h1) and sm/600 in `fg-muted` (h2/h3), following Granola's small, bold section heads.
  - Bold inside notes is 600.
- **Numbers:** `font-variant-numeric: tabular-nums` on timers, durations, timestamps and counts.

### 2.5 Spacing, size, radius

- **Spacing scale** (4px base): `0, 2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 64`.
- **Page gutters:** 24px horizontally (32px at ≥1280px).
- **Section gap:** 32px. **Paragraph/block gap in notes:** 4px plus line-height.
- **Control heights:** `24` (xs: chips, segmented), `28` (sm buttons, icon buttons), `32` (md default: buttons, inputs), `40` (lg: onboarding, chat bar inner) and `44` (chat bar).

**Radii:**

| Token | px | Use |
|---|---|---|
| `--radius-xs` | 4 | kbd, inline code, skeleton lines, citation chips |
| `--radius-sm` | 6 | nav items, menu items, icon buttons |
| `--radius-md` | 8 | buttons, inputs, selects, tooltips (the plan's default) |
| `--radius-lg` | 12 | popovers, menus, dialogs, toasts, palette, settings cards, bubbles |
| `--radius-xl` | 16 | right-panel cards, expanded record pill |
| `--radius-full` | 9999 | pills: tag pill, chat bar, collapsed record pill, avatars, segmented track |

### 2.6 Shadows (floating layers only)

| Token | Light | Dark |
|---|---|---|
| `--shadow-xs` (secondary button, segmented thumb) | `0 1px 2px rgb(31 31 28 / .06)` | `0 0 0 1px rgb(255 255 255 / .04)` |
| `--shadow-popover` | `0 0 0 .5px rgb(31 31 28 / .10), 0 4px 12px -2px rgb(31 31 28 / .08), 0 12px 32px -8px rgb(31 31 28 / .12)` | `0 0 0 .5px rgb(255 255 255 / .10), 0 8px 24px -4px rgb(0 0 0 / .50)` |
| `--shadow-dialog` | `0 0 0 .5px rgb(31 31 28 / .10), 0 24px 64px -12px rgb(31 31 28 / .24)` | `0 0 0 .5px rgb(255 255 255 / .10), 0 24px 64px -12px rgb(0 0 0 / .70)` |
| `--shadow-chatbar` | `0 0 0 .5px rgb(31 31 28 / .08), 0 6px 20px rgb(31 31 28 / .08)` | `0 0 0 .5px rgb(255 255 255 / .08), 0 8px 24px rgb(0 0 0 / .45)` |
| `--shadow-pill` | `0 0 0 .5px rgb(0 0 0 / .12), 0 8px 24px -6px rgb(0 0 0 / .20)` | `0 0 0 .5px rgb(255 255 255 / .12), 0 8px 24px -6px rgb(0 0 0 / .60)` |

In dark mode, the inner hairline ring does the edge work, because a shadow alone vanishes on charcoal (Granola and Circleback both rely on hairlines in dark).

### 2.7 Motion

| Token | Value | Use |
|---|---|---|
| `--dur-instant` | 80ms | Reduced-motion fallback (opacity only) |
| `--dur-fast` | 120ms | Hover, colour, icon swap, row hover background |
| `--dur-base` | 160ms | Popover, menu, tooltip, tab indicator slide, sidebar collapse |
| `--dur-slow` | 200ms | Dialog, palette, right panel, chat bar expand, toast enter |
| `--dur-pill` | 240ms | Record pill collapsed ↔ expanded morph (width + radius) |
| `--ease-out` | `cubic-bezier(0.22, 1, 0.36, 1)` | Everything entering or changing |
| `--ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | Exits (use `0.75 ×` the duration) |
| `--ease-in-out` | `cubic-bezier(0.65, 0, 0.35, 1)` | Skeleton pulse, level-meter idle wave |

- **Enter recipes:**
  - Popover: `opacity 0→1` with `translateY(4px)→0` and `scale(.98)→1`.
  - Dialog: `opacity` with `scale(.98)→1`.
  - Toast: `translateY(8px)`.
- **Never animate layout of long lists.**
- **Skeleton:** opacity 1 ↔ 0.55, 1.4s `--ease-in-out`, infinite.
- **`@media (prefers-reduced-motion: reduce)`:** set all durations to `--dur-instant`, transforms to none, and turn off the skeleton pulse and level-bar animation (show a static level).

### 2.8 Z-index layers

| Token | Value | Layer |
|---|---|---|
| `--z-base` | 0 | Content |
| `--z-sticky` | 10 | Sticky date headers, table headers |
| `--z-sidebar` | 20 | Sidebar and its resize handle |
| `--z-header` | 30 | Title bar and drag region |
| `--z-chatbar` | 40 | Floating chat bar, "Jump to now" |
| `--z-popover` | 50 | Dropdowns, popovers, hover cards, citation previews |
| `--z-overlay` | 60 | Scrims |
| `--z-dialog` | 70 | Dialogs, sheets |
| `--z-palette` | 80 | Command palette |
| `--z-toast` | 90 | Toasts |
| `--z-tooltip` | 100 | Tooltips |

The record pill is a separate Tauri window, so it needs no z-index.

### 2.9 Theme wiring

- **Main window:** use `next-themes` (MIT): `attribute="class"`, `defaultTheme="system"`, `enableSystem`, `disableTransitionOnChange`. It injects the no-flash head script.
  - **Alternative:** port anarlog's tiny boot script (`apps/desktop/public/theme-boot.js` + `src/shared/theme/{apply,resolve,provider}.tsx`, MIT).
- **Tauri:**
  - On change, call `getCurrentWindow().setTheme(null | 'light' | 'dark')` so native chrome and the traffic lights match. `null` means follow the system.
  - Listen to `onThemeChanged` and broadcast to the `record_panel` window (Handy emits `theme-changed`; anarlog's `provider.tsx` does both).
  - Set each window's native `backgroundColor` to `--bg` and create windows hidden, then show them after the first paint. This avoids a white flash in dark mode (a Cap pattern; re-implement it, don't copy it).
- **Settings → Appearance:** a 3-card picker (System / Light / Dark) with mini previews. The System card is split diagonally, as in anarlog `settings/appearance/theme.tsx`.

---

## 3. Component specs

All sizes are in px. "Light / Dark" notes appear only where behaviour differs beyond tokens.

### Sidebar (FND-10 shell)

- **Width:** 232 default, resizable 200–320 (persisted), collapsible to 0 with `[`. Collapse animates `opacity` and `translateX(-12px)` over 160ms (anarlog). Background `--bg-subtle`, right edge 1px `--border`. In dark, the sidebar (`#151513`) is darker than the canvas, following Granola's sunken / Spacedrive's sidebar 7% pattern.
- **Top (44px drag region):**
  - Traffic lights inset (x 16, y 16).
  - Right-aligned 28px ghost icon buttons: sidebar toggle, and new note (⌘N).
- **Search row:** 30px high, `--bg-muted` fill with no border, r6, 8px inset. Contents: 16px lucide `search` icon, placeholder "Search" in `fg-subtle`, and `⌘K` kbd on the right. Clicking it opens the palette (as in Granola).
- **Primary nav** (order from §4.6): Home, Meetings, Action Items, People, Companies, Insights, Recipes, Automations. Tags are reachable from the palette and Meetings filters.
- **Sections:**
  - Labels: 12/600 `fg-muted`, 24px tall, 8px left padding, 12px top margin. Optional collapse chevron on hover.
  - "Folders" is a tree; nested items indent 12px per level with a 16px disclosure.
- **Bottom (pinned):**
  - Settings nav item.
  - The **record block**: a 40px row that shows "Start recording ⌘⇧R" when idle (secondary button style, full width).
    - While recording it turns into a live row:
      - `--accent-soft` background, r8;
      - 5 accent level bars, the meeting title truncated, and the elapsed `12:04` in tabular `fg-muted`;
      - on hover, a 24px stop button in the `--recording` glyph.
    - Clicking the row opens the live meeting.
- **States:**
  - Hover row: `--bg-muted`.
  - Active row: `--bg-muted` + `fg` text + `fg` icon.
  - Idle row: `fg-muted` text and icon.
  - Focus: 2px focus ring, inset.

### Nav item

- **Size:** 30px tall, r6, padding 0 8. Icon 16px lucide (stroke 1.5), with an 8px gap to the 14/400 label.
- **Badge:** optional trailing count, 12/400 `fg-subtle`, tabular (Circleback shows a "2" on Action Items).
- **Transitions:** `background-color` and `color` over 120ms.
- **Drag-and-drop target** (folders): 1px inset `--accent` ring and an `--accent-soft` background.

### List row (Meetings, Recent notes, People, Action items)

- **One line (40px):**
  - padding 0 12, r8, gap 12;
  - optional leading 16px icon or 20px avatar;
  - title 14/400 `fg`, truncated;
  - trailing meta 12/400 `fg-muted`, tabular (time, duration, attendee avatars stacked at 16px with −4px overlap, folder name).
- **Two lines (56px):** title 14/400, plus a second line of 12/400 `fg-muted` (attendees · duration · first line of summary), following anarlog `sidebar/timeline/item.tsx`.
- **Groups:** sticky date-bucket headers ("Today", "Yesterday", "This week", then month names): 12/600 `fg-muted`, 32px tall, `--bg` background, `--z-sticky`.
- **States:**
  - Hover `--bg-muted`; selected `--bg-muted` plus `fg`.
  - Live meeting row: `--accent-soft` tint with level bars.
  - Upcoming: 12px `fg-muted` countdown "in 12m".
- **Row hover reveals** trailing ghost icon buttons (28px): Open, ⋯. Right-click opens the same menu.
- **Divider:** none between rows. Whitespace separates them.

### Page header

- **Content top:** 32px top padding below the 44px title bar.
- **Title:** 24/600 `fg`.
- **Optional:**
  - a subtitle, 14/400 `fg-muted`, 4px below;
  - right-aligned actions (md buttons, gap 8), aligned to the title's baseline row;
  - filter bar under the title: 32px, 16px gap. Filter chips are 28px, r-full, 1px `--border`, 12/400 `fg-muted`; active chips get an `--accent-soft` background and `fg` text.
- **Page width:** content max 960 for lists and 720 for documents, centred, with 24px gutters.

### Meeting header (meeting page)

- **Optional date badge** (Granola style): 32×32 r8 `--bg-muted`, with the month 10px uppercase `fg-muted` above the day 14/600. Skip it if it competes with the title.
- **Title:** 24/600, editable inline (EditableTitle). The caret is visible on hover via a `bg-muted` underline. While auto-title is pending, it shows a shimmer skeleton.
- **Meta chips row** (8px below the title, gap 6, wraps):
  - **Chip spec:** 24 high, padding 0 8, r-full, 1px `--border`, 12/400 `fg-muted`, 14px icon. Hover: `--bg-muted`.
  - **Chips:**
    - date (calendar icon, "Today 14:00");
    - duration (clock, "52m");
    - attendees (stack of up to 3 avatars at 16px, plus "+4");
    - folder;
    - tags (6px colour dot + label);
    - a ghost "+ Add tag" chip that shows only on hover.
  - These patterns match Circleback ("Today · 52m · ● Planning · Add tag") and Granola ("Today · 👥4 · folder").
- **Right-aligned actions:** Share/Export (secondary sm), Enhance or Regenerate (ghost sm with a sparkles icon), and ⋯ menu. While recording, a stop button and the elapsed time appear.
- **Tabs row:** 16px below the header (see Tabs).

### Tabs

- **Underline tabs** (Circleback): 36px tall, 14/400, `fg-muted` idle, `fg` with weight 600 when active.
  - Indicator: 2px `--fg` bar at the bottom, sliding over 160ms.
  - Tab gap: 20px.
  - The row has a 1px `--border` bottom line spanning the content width.
- **Tabs:** Notes, Transcript, Insights, Clips.
- **Notes sub-switch** (Enhanced / My notes) is a **segmented pill** placed at the right of the tab row (anarlog `header-shared.tsx`; Granola's "My notes | Enhanced" toggle):
  - track 28px tall, padding 2, r-full, `--bg-muted`;
  - segment 24px, 12/400 `fg-muted`; active segment `--surface-raised` + `fg` + `--shadow-xs`;
  - thumb slides over 160ms.
- **Keyboard:** ⌘1…⌘4 switch tabs, and ⌘E toggles Enhanced / My notes.

### Buttons

| Variant | Light | Dark | Hover | Active | Disabled |
|---|---|---|---|---|---|
| **Primary** | bg `--fg`, text `--bg` | bg `--fg` (`#EDEDE9`), text `#1A1A18` (Granola's dark `fill-primary #EAEBE5`) | `opacity .88` | `opacity .80` + `translateY(.5px)` | 40% opacity |
| **Secondary** | bg `--surface`, 1px `--border`, `--shadow-xs`, text `fg` | bg `--bg-muted`, border `--border-strong` | bg `--bg-subtle` | bg `--bg-muted` | — |
| **Ghost** | transparent, text `fg-muted` | (same) | bg `--bg-muted`, text `fg` | — | — |
| **Accent** (rare: "Join & record", onboarding CTA) | bg `--accent`, text `--accent-fg` | (same tokens) | — | — | — |
| **Danger** | ghost with `--danger` text; filled `--danger` only inside confirm dialogs | — | — | — | — |

- **Sizes:**
  - sm 28 (padding 0 10, 14/400, 16px icon, gap 6)
  - md 32 (padding 0 12)
  - lg 40 (padding 0 16, 14/600)
  - icon-only 28×28 r6
- **Radius:** r8.
- **Focus-visible:** 2px `--focus-ring` with 2px offset (offset colour `--bg`).
- **Loading:** label stays in place; a 14px spinner replaces the icon after 400ms.
- **Tooltips:** always on icon-only buttons, with the label and `Kbd`.

### Inputs (text, select, textarea)

- **Size:** 32 high, padding 0 10, r8, bg `--surface`, 1px `--border`, 14/400 `fg`, placeholder `fg-subtle`.
- **States:**
  - Hover: border `--border-strong`.
  - Focus: border `--accent` plus a 3px ring of `--accent` at 20% (`box-shadow`).
  - Error: border `--danger`, 12/400 `--danger` message 4px below, starting with an alert icon.
  - Disabled: bg `--bg-muted`, text `fg-subtle`.
- **Dark:** surface `#20201E` on bg `#1A1A18`; the 1px border carries the edge.
- **Select:** same box, with a chevron-down 16px `fg-muted`; the menu uses the popover spec.
- **Switch:** 32×18 track, 14px thumb; off `--border-strong`, on `--accent`; thumb `#fff` (dark: `#EDEDE9`); 120ms.
- **Checkbox:** 16px r4, on `--accent`.

### Popover / dropdown menu / context menu

- **Container:** `--surface-raised`, r12, `--shadow-popover`, padding 4, min-width 200, max-height 360 (scroll with `mask-fade-out` at the edges, the Spacedrive utility).
- **Items:**
  - 30 high, r6, padding 0 8, 14/400 `fg`, 16px icon `fg-muted`;
  - trailing kbd or `fg-subtle` meta;
  - hover/highlight `--bg-muted`;
  - destructive items `--danger`.
- **Group label:** 12/600 `fg-muted`, padding 6 8 2.
- **Separator:** 1px `--border`, 4px vertical margin.
- **Motion:** 160ms enter from the trigger side (4px).
- **Dark:** raised `#2A2A27` is lighter than the surface, and the hairline ring defines the edge.

### Dialog

- **Size and container:** width 440 (sm) / 560 (md) / 720 (lg, e.g. template editor); r12; `--surface-raised`; `--shadow-dialog`; padding 24.
- **Content:** title 16/600, description 14/400 `fg-muted` 4px below, body 16px below. Footer is right-aligned with an 8px gap: Cancel (ghost) then the primary action, or Danger for destructive actions.
- **Scrim:** `--overlay`, no blur.
- **Behaviour:** Esc closes; focus is trapped. 200ms `scale(.98)` plus opacity.
- **Confirmations** are reserved for destructive or irreversible actions (e.g. "Regenerate will replace your edits", NOT-02).

### Toast (sonner, restyled)

- **Position and size:** bottom-right, 16px inset (bottom-centre above the chat bar on the meeting page); width 360; r12; `--surface-raised`; `--shadow-popover`; padding 12 14.
- **Content:** 16px status icon (success `--success`, error `--danger`, warning `--warning`, info `fg-muted`); title 14/600; optional description 14/400 `fg-muted`; optional action (ghost sm, e.g. "Undo").
- **Durations:** 4s, 6s for errors with an action, and persistent for progress (model download). Progress toasts show a 2px `--accent` bar at the bottom.
- **Stacking:** at most 3 visible; the rest collapse into a stack with 8px offsets.

### Command palette (⌘K, cmdk)

- **Container:** width 640, top at 18vh, r12, `--surface-raised`, `--shadow-dialog`, scrim `--overlay` at 50% strength.
- **Input:** 48 high, padding 0 16, 16/400, no border, 20px search icon; 1px `--border` under it.
- **Scope chip:** optional, shown when scoped (e.g. "in Q4 roadmap").
- **List:** max-height 400, padding 6.
  - Group labels 12/600 `fg-muted` (Navigation, Actions, Meetings, People).
  - Items 36 high, r6, padding 0 10, 16px icon, 14/400 label, trailing 12 `fg-subtle` meta or kbd; selected `--bg-muted`.
- **Footer:** 32 high, 1px top border, 12/400 `fg-subtle` hints ("↵ Open · ⌘↵ Open in new window · esc").
- **Results:**
  - Meeting results show the title, plus date and a matched snippet with `--highlight` on the hit.
  - Results stream progressively, so no blocking spinner is needed.

### Chat bar (AI-02 floating "Ask anything", ⌘J)

- **Resting:**
  - Bottom-centre and floating: width `min(640px, content − 48px)`, 16px above the window bottom.
  - 44 high, r-full, `--surface`, `--shadow-chatbar`.
  - Left: a scope chip (24 high, r-full, `--bg-muted`, 12/400 `fg-muted`, e.g. "This meeting ▾").
  - Placeholder "Ask anything" 14/400 `fg-subtle`.
  - Right: model picker ("Auto ▾" 12/400 `fg-muted`, Granola style), a mic button (28 circle, ghost), and send (28 circle; disabled `--bg-muted`, enabled `--fg` fill with an arrow-up in `--bg`).
  - On the meeting page, recipe chips ("Write follow-up email", "List actions") show inside the bar on the right when it is empty, as in Granola.
- **Focused / typing:** grows upward into a panel. r16, max-height 480, 200ms. Multiline input at 14/20; Enter sends, ⇧Enter adds a newline.
- **Conversation layout** (in the panel, the right panel or the full page):
  - User turns: right-aligned bubble, max 80%, `--bg-muted`, r12, padding 8 12, 14/20 `fg`.
  - Assistant: plain text, no bubble, full width, 14/22 (16/26 on the full page).
  - Tool and "thinking" steps: 12/400 `--ai-text` lines with a 14px icon (search, file); collapsed into "Thought for 4s ▸" (Granola chat and Gumloop show the same pattern).
  - Citation chips: 18 high, r4, `--bg-muted`, 12/400 `fg-muted`, e.g. `[Q4 sync · 12:04]`. Hover shows a hover card preview (popover spec); click jumps to that timestamp.
- **During recording on the meeting page,** the bar sits beside the record status: Granola shows the "Transcribing" dancing-bars pill at left with Ask anything at right in the same bottom slot. Do the same.

### Transcript bubble (TRN-01)

- **Turn grouping:** consecutive segments from the same speaker within 60s make one turn.
- **Turn header** (shown once per turn, 4px above the first bubble):
  - speaker chip;
  - timestamp 12/400 `fg-subtle`, tabular (`12:04`), on the same line. It sits right of the name for "them" and left of the name for "me".
- **Bubble:**
  - max-width 75% of a 720 column; padding 6 10; 14/20; r12; the corner nearest the speaker side is r4;
  - 2px gap between bubbles in one turn; 16px gap between turns.
- **Me** (right-aligned): bg `--accent-soft`, text `fg`. TRN-01 says "accent colour". We use the soft tint rather than Granola's solid teal with white text, because 5,000 solid bubbles shout. Offer "solid" as a setting if wanted.
- **Them** (left-aligned): bg `--bg-muted`, text `fg`. The speaker name is coloured; the bubble stays neutral.
- **Live partial text:** `fg-muted` italic at 0.6 opacity (anarlog). The final text replaces it without a layout jump.
- **Hover:**
  - A 28-high floating toolbar (`--surface-raised`, r8, `--shadow-popover`) at the bubble's top outer corner, with: timestamp, Play from here, Copy, Clip, Comment, Delete (Granola shows `00:00 · copy · delete`).
  - Hover background `--bg-muted` at 60%, over 120ms.
- **Search (⌘F):**
  - Hits use `--highlight`; the current hit uses `--selection` plus a 1px `--accent` outline.
  - Sticky search bar 36 high at the top of the tab, with the "3 of 27" counter and `speaker:` token chips.
- **"Jump to now"** (live mode, scrolled up): pill 32 high, `--fg` fill, `--bg` text, 12/600, bottom-centre above the chat bar, with a 12px arrow-down.
- **Virtualise** with `@tanstack/react-virtual`, using measured row heights and keying by turn.

### Speaker chip

- **Name only:** 14px avatar circle (initials 9/600, background = speaker colour at 16%, text = speaker colour) followed by the name in 12/600 speaker colour.
- **Inline in notes** (@mention): 20 high, r-full, `--bg-muted`, padding 0 6 0 2, 16px avatar, 14/400 `fg`.
- **Behaviour:** click opens the "Assign speaker" popover (rename, merge, "This is me"). Unknown speakers show "Speaker 2" in `fg-muted` with a dashed 1px avatar outline.

### Tag pill

- **Spec:** 22 high, padding 0 8, r-full, 1px `--border`, 12/400 `fg-muted`, with a leading 6px dot in the tag colour (tag colours reuse the speaker palette).
- **States:** hover `--bg-muted`; removable variant shows a 12px × on hover.
- **Inside dense rows:** dot + label with no border.

### Record pill (CAP-10 `record_panel` window)

**Window**
- Separate Tauri window: transparent, no decorations, no shadow (shadow drawn in CSS), always-on-top, skips the taskbar, visible on all Spaces and fullscreen, **non-activating NSPanel** so it never steals focus (Handy `src-tauri/src/overlay.rs` with `tauri-nspanel`; `macOSPrivateApi: true`).
- Fixed window size 360×72, never resized. The pill morphs *inside* the window, because resizing transparent on-top windows flashes a stale frame (insight from openwhispr `windowConfig.js`).
- Click-through outside the pill via `set_ignore_cursor_events` toggled on pointer enter and leave of the pill bounds.
- **Placement:** default is right screen edge, vertically centred at one third (Granola's "nub"). Draggable, snaps to the nearest edge or corner with a 16px inset, and remembers its position.

**Collapsed** (resting while recording)
- Size and surface: 112×36, r-full, `--surface-raised` at 92% with `backdrop-filter: blur(20px) saturate(140%)`, `--shadow-pill`.
- Layout, padding 0 12, gap 8:
  - **7px `--recording` dot.** Opacity pulses 1 → .45 over 1.8s; static with reduced motion.
  - **Level meter:** 5 bars, 3px wide with a 2px gap, 4–16px tall, r2, `--accent`.
    - Heights follow mic level with EMA smoothing (`0.7·prev + 0.3·new`, Handy) at about 30fps; the transition is 80ms linear.
    - System-audio activity tints the outer two bars at 60% (one meter, two sources).
  - **Elapsed time** `12:04`, 12/600 tabular `fg`.

**Expanded** (pointer hover for 150ms, or focus)
- Size: 332×44, r22. Width and radius animate over 240ms `--ease-out`.
- Adds, after the timer:
  - divider (1px × 16 `--border`);
  - Pause/Resume (28 icon button);
  - Stop (28 circle with the `--recording` square glyph);
  - mic picker (28 icon button, opens a popover menu rendered within the window bounds, i.e. upward or sideways);
  - system-audio toggle (28 icon button, accent when on);
  - "Open notes" (ghost sm, 12px label).
- Tooltips show shortcuts. Leaving collapses it after a 400ms delay.

**State variants**

| State | Treatment |
|---|---|
| **Paused** | Dot turns `fg-subtle` and stops pulsing; bars freeze at 4px; timer is `fg-muted`; the border becomes 1px `--warning` (Cap's paused-amber pattern) |
| **Health warning** | Bars `--warning`. Pill auto-expands to 300×36 with a 12/400 `--warning` label ("No mic audio for 5s", "Low disk"), plus a macOS notification |
| **Starting / arming** | Bars run a travelling `scaleY` wave over 900ms with a 75ms stagger, in `fg-subtle` (Handy's arming state) |
| **Stopping / processing** | Bars give way to a 12px spinner and "Saving…" |

**Interaction**
- Clicking the body (not a control) focuses the main window on the live meeting.
- Dark and light are both fine: the pill uses the theme tokens, and its blur keeps it legible over any wallpaper.

### Skeleton

- **Spec:** `--bg-muted` blocks, r4, pulse as in §2.7. A text line is 10px high for 14px text and 12px for 16px text, with 10px vertical gaps; vary widths (100/92/76/60%).
- **Shapes per screen:**
  - List rows: 20px circle + 2 lines.
  - Meeting title: 24px × 40% block.
  - Enhanced notes: heading block plus 4 bullet lines.
- **Rules:** show a skeleton only after 150ms (avoid flicker) and keep it for at least 300ms once shown.

### Empty state

- **Layout:** centred in the content area at about 38% from the top, max-width 360, text-centred.
- **Content:**
  - Icon: 20px lucide, stroke 1.5, `fg-muted`, inside a 40px r12 `--bg-muted` square.
  - Title: 14/600 `fg`, 12px below.
  - Body: 14/400 `fg-muted`, 4px below, at most 2 lines.
  - One action: secondary md button with a trailing `Kbd`, 16px below.
- **Examples:**
  - Meetings: "No meetings yet" + "Start recording ⌘⇧R".
  - Action items: "Nothing due" + "Open last meeting".
  - Transcript (not recording): "No transcript yet" + "Start recording".
  - Search: "No results for 'x'" + "Search transcripts instead".
- **Home with no calendar connected:** the empty state is a list of actions with shortcuts (anarlog `main/empty.tsx`): New note ⌘N, Start recording ⌘⇧R, Connect calendar, Settings ⌘,.

### Settings layout

- **Route:** a full-window route (not a modal; the current `SettingsModal` goes away).
- **Settings nav:** 200px, `--bg-subtle`, 44px drag region, title "Settings" 16/600. Nav items follow the nav-item spec.
- **Groups** (labels 12/600 `fg-muted`):

  | Group | Pages |
  |---|---|
  | App | General, Appearance, Notifications, Shortcuts |
  | Capture | Recording & Audio, Transcription, Speakers & Voices, Vocabulary |
  | AI | Models & Providers, Templates, Profile & Context |
  | Workspace | Calendar, Connections, Automations, Folders & Tags |
  | Data | Privacy & Data, Import / Export, Storage |
  | About | — |

- **Footer:** version 12/400 `fg-subtle` (click to copy).
- **Content:** max-width 640, padding 32 24 64, page title 24/600, 32px gap between sections (Spacedrive `Settings/Layout.tsx`; Cap uses the same proportions).
- **Section:**
  - Title 14/600 `fg` *outside* the card, an optional 12/400 `fg-muted` description, then 8px gap.
  - Card: `--surface` (dark `--bg-muted`), 1px `--border`, r12, `divide-y` `--border`.
- **Setting row:**
  - min-height 52, padding 12 16, grid `1fr auto`;
  - label 14/400 `fg`; description 12/400 `fg-muted` (at most 2 lines);
  - the control on the right is 200px wide for selects and inputs; switches are right-aligned.
- **Nested detail rows** (e.g. a model with download progress) expand inline with a 160ms height animation.
- **Destructive rows** sit in a separate final "Danger zone" card: `--danger` text button and a confirm dialog.
- **Connections page:** one row per provider with a status dot (`--success` / `--warning` / `fg-subtle`), "Last synced 4m ago" in 12 `fg-muted`, and an error message inline in `--danger`.

---

## 4. Screen blueprints

Width assumption: a 1280px window.

### 4.1 Home (ORG-02)

```
┌──────────────┬──────────────────────────────────────────────────────────────┐
│ ● ● ●   ⊟ ✎  │                                                    (44 drag) │
│ ┌──────────┐ │                                                              │
│ │⌕ Search ⌘K│ │   Good afternoon                                    24/600   │
│ └──────────┘ │                                                              │
│ ⌂ Home       │   COMING UP                          12/600 fg-muted         │
│ ▤ Meetings   │   ┌────────────────────────────────────────────────────────┐ │
│ ☑ Action  3  │   │ 14:00  Q4 roadmap sync     👤👤👤+2   in 12m  [Join ●] │ │ 40px rows
│ 👤 People    │   │ 15:30  1:1 Priya           👤          15:30   [ ○ rec] │ │ hairline-free
│ ▦ Companies  │   │ Tomorrow                                               │ │ sticky bucket
│ ✦ Insights   │   │ 09:00  Weekly standup      👤👤👤+6   09:00   [ ○ rec] │ │
│ ⚡ Recipes   │   └────────────────────────────────────────────────────────┘ │
│ ⟳ Automations│                                                              │
│              │   RECENT NOTES                                   View all →  │
│ FOLDERS    ⌄ │   Q3 GTM sync                     Today 11:02 · 34m · Sales  │ 56px 2-line
│  ▸ Sales     │   Agreed to narrow Q3 focus to mid-market…          fg-muted │
│  ▸ Hiring    │   Design crit                      Yesterday · 48m           │
│              │                                                              │
│              │   SUGGESTED FOLLOW-UPS            OPEN ACTION ITEMS (due)    │
│              │   ✉ Recap to Northwind  [Draft]   ○ Send roadmap   Fri  JB   │
│──────────────│   ✉ Thank-you to Priya  [Draft]   ○ Spec API       Mon  SR   │
│ ⚙ Settings   │                                                              │
│┌────────────┐│                                                              │
││● Start rec ⌘⇧R│                ╭──────────────────────────────────╮        │
│└────────────┘│                 │ This meeting▾ Ask anything  Auto ◎ ↑│ chat bar
└──────────────┴─────────────────╰──────────────────────────────────╯────────┘
  232px            content max 960, gutters 24
```

### 4.2 Meeting page: Notes tab, chat in the right panel (`]`)

```
┌──────────────┬───────────────────────────────────────────────┬───────────────┐
│ sidebar      │ ‹ Meetings                    ⤴ Share  ✦  ⋯   │ Chat  People ⓘ│ 44 drag
│              │                                               │───────────────│
│              │  Q4 roadmap alignment                24/600   │ Ask about this│
│              │  [▣ Today 14:00][◷ 52m][👤👤👤+2][▢ Sales]     │ meeting…      │
│              │  [● Planning][+ tag]               chips 24px │               │
│              │                                               │   What did   ▕│ user bubble
│              │  Notes   Transcript   Insights   Clips  (Enhanced|My notes)    │ right, muted
│              │  ━━━━━                                  segmented pill        │
│              │ ─────────────────────────────────────────────  │ ⌕ Read notes  │ ai-text steps
│              │                                               │ Sofia owns the│
│              │  Action items                       16/600    │ API spec, due │
│              │  ○ Send updated roadmap   [JB] Fri            │ Fri [Q4·12:04]│ citation chip
│              │  ○ Draft API spec         [SR] Mon            │               │
│              │                                               │               │
│              │  Overview                                     │               │
│              │  • narrow Q3 to mid-market       ← user-text  │               │
│              │     – SMB deprioritised for Q3   ← ai-text    │               │
│              │     – paid paused until ICP doc  ← ai-text    │               │
│              │  • push notifications first      ← user-text  │               │
│              │     – retention −40% without push[12:04]      │╭─────────────╮│
│              │                                               ││ Ask… ◎   ↑  ││
│              │            column 720, 16/26                  │╰─────────────╯│
└──────────────┴───────────────────────────────────────────────┴───────────────┘
                                                                 320px (280–420)
```

- A table of contents floats at the right edge of the note column when there are three or more headings and the right panel is closed: 12/400 `fg-muted`, active heading `fg`.

### 4.3 Recording in progress (meeting page + floating pill)

```
┌──────────────┬──────────────────────────────────────────────────────────────┐
│ …            │ ‹ Meetings          ● 12:04  [❚❚] [■ Stop]   ⋯               │
│ ⚙ Settings   │                                                              │
│┌────────────┐│  Northwind sync                                              │
││▮▯▮▮▯ Northw… 12:04 ■│  [▣ Today][👤 2]                                     │
│└────────────┘│  My notes   Transcript                                       │
│ live row     │  ━━━━━━━━                                                    │
│ accent-soft  │  walked thru q3 priorities                 ← user typing     │
│              │  2 pulled forward|                           (BlockNote)     │
│              │                                                              │
│              │                                                              │
│              │      ╭──────────────╮ ╭──────────────────────────────╮       │
│              │      │ ▮▯▮▮▯ Transcribing ▾│ │ Ask anything     Write follow-up│
│              │      ╰──────────────╯ ╰──────────────────────────────╯       │
└──────────────┴──────────────────────────────────────────────────────────────┘
   Clicking "Transcribing ▾" opens a live transcript drawer (bubbles), 40% height.

Screen edge (other apps in front), record_panel window:
                                                     ╭───────────────╮
  collapsed 112×36 ────────────────────────────────▶ │ ● ▮▯▮▮▯ 12:04 │
                                                     ╰───────────────╯
  hover-expanded 332×44:
           ╭────────────────────────────────────────────────────────╮
           │ ● ▮▯▮▮▯ 12:04 │ ❚❚  ■  🎙▾  🔊  Open notes            │
           ╰────────────────────────────────────────────────────────╯
  warning:  ╭───────────────────────────────────╮  bars in --warning
            │ ● ▮▯▯▯▯ 12:04  No mic audio · 5s  │
            ╰───────────────────────────────────╯
```

### 4.4 Meetings list

```
┌──────────────┬──────────────────────────────────────────────────────────────┐
│ sidebar      │                                                              │
│              │  Meetings                                     [+ New ⌘N]     │
│              │  [All ▾] [Folder ▾] [People ▾] [Tag ▾] [⌕ Filter…]   28px    │
│              │                                                              │
│              │  TODAY                                     sticky, 12/600    │
│              │  ▤ Q4 roadmap alignment   👤👤👤+2  Sales   14:00   52m  ⋯   │ 40px
│              │  ▤ Design crit            👤👤      Design  11:00   48m      │
│              │  YESTERDAY                                                   │
│              │  ▤ 1:1 Priya              👤        —       16:30   31m      │
│              │  ▤ Northwind sync         👤👤      Sales   10:00   44m      │
│              │  SEPTEMBER                                                   │
│              │  …                                                           │
│              │  (multi-select: ⌘-click → bottom action bar: Move · Tag · Export · Delete)
└──────────────┴──────────────────────────────────────────────────────────────┘
 columns: title flex · attendees 96 · folder 96 · time 56 · duration 48 · actions 28
```

### 4.5 Person page

```
┌──────────────┬──────────────────────────────────────────────────────────────┐
│ sidebar      │                                                              │
│              │  (AP)  Alex Park                                   24/600    │
│              │  48px  VP Product · Northwind   alex@northwind.com   fg-muted│
│              │        [Email] [Ask about Alex ⌘J]  [⋯]                      │
│              │                                                              │
│              │  Overview   Meetings 12   Action items 3   Notes             │
│              │  ━━━━━━━━                                                    │
│              │  ABOUT (AI)                              ai-text, editable   │
│              │  Pushed back on pricing in the last two syncs; Q3 timeline   │
│              │  is a hard constraint. Prefers async follow-ups.             │
│              │                                                              │
│              │  RECENT MEETINGS                                             │
│              │  ▤ Northwind sync            Yesterday   44m                 │
│              │  ▤ Pricing review            Sep 28      30m                 │
│              │                                                              │
│              │  OPEN ACTION ITEMS                                           │
│              │  ○ Send counter-proposal       Me       Fri                  │
│              │  ○ Share EU residency doc      Alex     —                    │
└──────────────┴──────────────────────────────────────────────────────────────┘
```

### 4.6 Settings

```
┌────────────────┬────────────────────────────────────────────────────────────┐
│ ● ● ●          │                                                            │
│ Settings       │  Recording & Audio                               24/600    │
│                │                                                            │
│ APP            │  Input                                     14/600          │
│  General       │  ┌──────────────────────────────────────────────────────┐  │
│  Appearance    │  │ Microphone                       [MacBook Mic    ▾]  │  │ 52px row
│  Notifications │  │ Used for "Me" channel             12 fg-muted        │  │
│  Shortcuts     │  ├──────────────────────────────────────────────────────┤  │
│ CAPTURE        │  │ Capture system audio                         [●━━]   │  │
│ ▸Recording     │  ├──────────────────────────────────────────────────────┤  │
│  Transcription │  │ Input level                     ▮▮▮▮▮▯▯▯▯▯           │  │
│  Speakers      │  └──────────────────────────────────────────────────────┘  │
│  Vocabulary    │                                                            │
│ AI             │  Auto-record                                               │
│  Models        │  ┌──────────────────────────────────────────────────────┐  │
│  Templates     │  │ Internal meetings                            [●━━]   │  │
│ WORKSPACE      │  │ External meetings                            [━━○]   │  │
│  Calendar      │  └──────────────────────────────────────────────────────┘  │
│  Connections   │                                                            │
│ DATA           │  Danger zone                                               │
│  Privacy       │  ┌──────────────────────────────────────────────────────┐  │
│  Storage       │  │ Delete all recordings          [Delete…] (danger)    │  │
│ v0.9.2         │  └──────────────────────────────────────────────────────┘  │
└────────────────┴────────────────────────────────────────────────────────────┘
   200px nav         content max 640
```

---

## 5. What to reuse, with licences

### 5.1 Reference code (MIT or Apache only; keep copyright and NOTICE headers when porting)

**anarlog** (MIT, except `enterprise/**`, which is commercial: **do not copy**. `apps/desktop/src/settings/stats/tremor/` is Apache-2.0.)

| Path | Why |
|---|---|
| `packages/design-system/src/tokens.css` | Warm-stone light/dark token set. Validates our warm-neutral direction; their bg `#FAFAF9` / fg `#1C1917`, dark bg `#1F1C1A`. |
| `apps/desktop/public/theme-boot.js` + `src/shared/theme/{apply,resolve,provider}.tsx` | System/light/dark with no flash and Tauri `setTheme` and `onThemeChanged` sync. |
| `packages/editor/src/styles/prosemirror/note-typography.css` | Note typography: proportional headings, 120ms hover-row backdrop with negative inset, and the "streaming view = editor" pixel-stable trick. Port the CSS into our BlockNote theme. |
| `apps/desktop/src/session/components/note-input/header.tsx` + `header-shared.tsx` | Segmented pill switch for raw / enhanced / transcript. |
| `apps/desktop/src/session/components/note-input/transcript/renderer/{segment,utils,virtual-segments}.tsx` | Speaker-coloured segments, OKLCH golden-angle palette, live-word italic, current-sentence highlight. Their layout has no bubbles; use it for logic only. |
| `packages/ui/src/components/ui/dancing-sticks.tsx` | Amplitude-driven bar meter for the sidebar live row and the record pill. |
| `apps/desktop/src/meeting-float/overlay/{bar.tsx,layout.ts}` + `plugins/windows/src/window/floating_bar.rs` | Floating bar geometry: compact height 38, radii 14 / 21, hover drag handle, live-caption expansion. |
| `apps/desktop/src/sidebar/timeline/{item,buckets}.tsx`, `src/main/{body.tsx,left-sidebar-panel.ts}` | Timeline rows with live and upcoming states, sticky buckets, resizable and collapsible sidebar (200–360, `react-resizable-panels`). |
| `apps/desktop/src/chat/components/persistent-chat.tsx`, `src/shared/chat-cta.tsx`, `src/chat/surface.ts` | Bottom pill CTA that expands to a floating chat with a `clipPath` reveal (0.18s, house easing). |
| `apps/desktop/src/main/empty.tsx`, `transcript/screens/{empty,listening}.tsx` | Empty states with shortcut lists. |
| `apps/desktop/src/settings/setting-row.tsx`, `src/settings/appearance/theme.tsx` | Settings row and the 3-card theme picker. |

Do **not** take `apps/desktop/public/fonts/SF-Pro-*` (Apple-licensed) or anarlog's Hugeicons wrapper (we standardise on lucide).

**Handy** (MIT): the best Tauri overlay reference.

| Path | Why |
|---|---|
| `src-tauri/src/overlay.rs` | `tauri-nspanel` non-activating panel (`PanelLevel::Status`, `can_join_all_spaces`, `full_screen_auxiliary`), monitor-under-cursor placement, generation-guarded hide. |
| `src/overlay/RecordingOverlay.{tsx,css}` | Pill state machine (arming / recording / transcribing), 9-bar meter, EMA smoothing, width and radius morph (we shorten Handy's 460ms to 240ms). |
| `src-tauri/src/audio_toolkit/audio/visualizer.rs` | FFT into 16 log buckets (400–4000Hz), adaptive noise floor, 33ms emit to the overlay window. Drives our level bars. |
| `src/styles/theme.css`, `src/lib/utils/theme.ts` | Light/dark pairs plus a `data-theme` override mirrored to localStorage. |

**openwhispr** (MIT, Electron; port the ideas)
- `src/helpers/windowConfig.js`: the "never resize the transparent overlay window" rule, and on-top level escalation.
- `src/components/dictation/{PillWaveform.tsx,waveformMath.ts}`: RMS-to-bar curve `4 + min(1,(rms·8)^0.75)·18` written straight to the DOM in rAF, with no React re-renders. Use this for the 30fps meter.
- `src/styles/dictation-panel.css` (~l.290–425): an `@property` conic "thinking" ring, an option for the processing state.

**muesli** (MIT; the vendored Qwen3ASR is Apache-2.0; native Swift, so these are patterns)
- `design-system/muesli-design-system.html`: semantic meeting-state colours (recording / transcribing / success) and a 720pt reading column, which confirms our layout.
- `native/MuesliNative/Sources/MuesliNativeApp/{FloatingIndicatorController,NotchIndicatorController,MeetingNotificationController}.swift`: edge anchoring with nine positions and an 8pt inset; a notch-wrap variant for a later iteration; the "meeting detected" prompt (ORG-02 notification window).

**Spacedrive** (Apache-2.0, keep the NOTICE)
- `packages/interface/src/Settings/Layout.tsx` (`SettingsPageContent` / `Section` / `SectionRows` / `SettingItem`) and `routes/settings/index.tsx`: the settings shell we specify in §3.
- `apps/tauri/src/index.css`: `mask-fade-out` (40px edge fade for scroll areas) and `frame` (1px masked inner border).
- Token *grammar*: context × role (`app-*`, `sidebar-*`, `menu-*` × `box`, `line`, `hover`, `selected`, `ink`, `ink-dull`, `ink-faint`), plus `.inspector-content-plane` context re-mapping.
  - Our tokens are simpler, but borrow the idea: the sidebar uses `--bg-subtle`, and nested surfaces re-map instead of using `dark:` overrides.
- `apps/tauri/crates/macos/src-swift/window.swift`: an invisible-NSToolbar trick to pad the traffic lights. Use it if `trafficLightPosition` isn't enough.

**Cap** (AGPLv3 for the whole desktop UI): **patterns only, copy nothing.** Its vendored `vendor/tauri-nspanel` is MIT/Apache, but take it from upstream instead. Patterns worth re-implementing:
- windows hidden until the first themed paint, with a native background colour;
- click-through transparent panels that report hit bounds to Rust;
- content protection only while recording;
- paused state shown by recolouring amber;
- a timer inside the stop button;
- a 200ms peak-hold mic meter.

### 5.2 Libraries

| Concern | Decision | Notes |
|---|---|---|
| **Font** | `@fontsource-variable/inter` (OFL), self-hosted | Remove `next/font/google` Source Sans 3. No network at runtime, deterministic builds. System SF stack as fallback. |
| **Icons** | `lucide-react` (already installed) | 16px, `strokeWidth={1.5}`, `currentColor`, colour from text tokens. 20px only inside empty-state tiles. One `Icon` wrapper to enforce size and stroke. |
| **Editor** | **Keep BlockNote** (0.36 already in Deino; NOT-01 / NOT-02 specify it) | See below. |
| **Motion** | **CSS first** (Tailwind transitions + `tailwindcss-animate` + the duration and easing tokens) | `framer-motion` (installed; v11 can be imported as `motion`) only where CSS can't: the chat bar → panel expand (height auto plus `clipPath`, as in anarlog), toast stacking (sonner handles it), and the sidebar list reorder on drag. The record pill is pure CSS in its own lightweight window (Handy proves it). Always gate with `useReducedMotion`. |
| **Theme** | `next-themes` (MIT) or anarlog's boot script | Plus Tauri `setTheme`. |
| **Palette** | `cmdk` (installed) | Restyle per §3. |
| **Toasts** | `sonner` (installed) | `toastOptions.classNames` mapped to tokens; `richColors` off. |
| **Virtualisation** | `@tanstack/react-virtual` (installed) | Transcript turns and long meeting lists. |
| **Resizable panels** | `react-resizable-panels` (MIT, used by anarlog and shadcn's Resizable) | Sidebar and right panel. |
| **Overlay window** | `tauri-nspanel` (MIT/Apache, upstream `ahkohd/tauri-nspanel` v2 branch) | Non-activating record panel. |
| **Markdown streaming** | `streamdown` (MIT, anarlog) or BlockNote's markdown import | For streaming chat answers and enhanced notes before they become editable. |

**Editor decision: BlockNote vs TipTap vs anarlog's choice**

- **What anarlog uses:** anarlog uses **neither**. Its editor is **raw ProseMirror** through `@handlewithcare/react-prosemirror ^3.2.6` (`packages/editor`), with `prosemirror-*`, `y-prosemirror` and `yjs` for collaboration, and `prosemirror-markdown` + `markdown-it`.
  - The repo still has a Rust `crates/tiptap` that converts TipTap-compatible JSON and Markdown. That is a sign it previously stored TipTap documents (Hyprnote era) and kept the format while dropping the TipTap runtime.
  - Its git history is shallow, so the reason isn't recorded. The code suggests it wanted full control over node views (task rows, mentions, app-link chips, comment anchors), React-19-safe rendering, CRDT sync, and one typography class shared by the editor and the `streamdown` streaming view.
  - Notably, anarlog shows AI and user text as **separate documents** (raw vs enhanced tabs) rather than inline origin marks. Its unused `mark[data-ai-highlight]` style is dead code.
- **Why Deino keeps BlockNote** (built on TipTap/ProseMirror):
  - It is already integrated.
  - It gives slash menu, drag handles, markdown shortcuts, checklists, image blocks and a shadcn skin for free.
  - NOT-02 needs per-span origin, which BlockNote supports through **custom styles** (`createReactStyleSpec`: `origin: "user" | "ai" | "ai-edited"`) or a custom inline content type, rendered with `color: var(--user-text)` / `var(--ai-text)`.
  - Raw ProseMirror would cost weeks for parity we don't need.
- **What to borrow from anarlog:** its `note-typography.css` rules into a BlockNote theme override (`.bn-container[data-color-scheme]` mapped to our tokens), and the pattern of rendering the streaming enhanced note with the same typography class so the swap to the editable document doesn't shift.
- **Cleanup:** remove the direct `@tiptap/*` dependencies unless something imports them.

---

## 6. Implementation checklist for FND-10 (derived)

1. **Replace the `globals.css` tokens** with §2 (channels format) and add the shadcn aliases. Map Tailwind colours, radii, shadows, z-index, durations and easings in `tailwind.config.js`. Delete the vibrate and fade-in-up keyframes (too bouncy and too long) and the hard-coded scrollbar greys (switch to `--border-strong`).
2. **Theming:** add `ThemeProvider` (system default) plus Tauri `setTheme` sync, with an Appearance setting.
3. **Primitives:** build `Icon`, `Kbd`, `EmptyState`, `PageHeader`, `Skeleton`, `Chip`/`TagPill`, `SegmentedControl`, `SettingsSection`/`SettingRow`. Restyle `button`, `input`, `select`, `switch`, `popover`, `dropdown-menu`, `dialog`, `tooltip`, `command` and `tabs` to the tokens.
4. **Shell:** sidebar from the `nav.ts` registry, the 44px drag title bar with a traffic-light inset, and resizable panels.
5. **Lint gate:** forbid raw palette classes and hex values in TSX, then migrate the ~1,069 occurrences screen by screen. "Dark mode has no unreadable text" can't pass until this is done.
6. **Screenshots:** capture each blueprint screen in `dev:mock` in both themes for the PR.

---

## 7. Open questions for the owner

- **Accent hue:** this spec proposes calm green `#2F6E4E` / `#7CC39A`, which harmonises with Granola's olive but stays distinct. The alternative is Deino brand colours if a logo palette exists.
- **Me-bubble style:** `--accent-soft` tint (recommended, calm), or Granola's solid accent with white text?
- **Font:** Inter Variable (consistent with the plan, slightly more "designed"), or the pure SF system stack (zero bytes, most native)?

---

## 8. Screenshots (`ui-shots/`)

All were taken from public marketing, changelog and demo pages. No sign-in, no forms; Granola's cookie banner was declined with "Reject all".

| File | What it shows |
|---|---|
| `granola-01-hero.jpg` | Homepage hero: note window mockup with title (serif marketing), Today · 4 attendees chips, minimal user notes |
| `granola-02-enhanced-notes-user-vs-ai.png` | **Enhanced note:** small bold section heads, user lines in ink, AI sub-bullets in grey; "My notes / Enhanced" segmented toggle at the bottom |
| `granola-03-transcribing-pill.jpg` | Note window during a call, with the **"Transcribing" dancing-bars pill** bottom-centre |
| `granola-04-before-meeting-brief.jpg` | "Your Brief" card under the note title before a meeting |
| `granola-05-in-meeting-pill.jpg` | In-meeting state: sparse user notes, transcribing pill, video call beside |
| `granola-06-after-meeting-recipes.jpg` | Post-meeting recipe list (List actions / Write follow-up email / Draft project plan) with soft green icon tiles |
| `granola-07-chat-ask.jpg` | Granola Chat input with the "Team stand-ups" scope chip, attach and controls icons |
| `granola-08-meeting-page-chatbar.jpg` | **Real app meeting page:** date badge, 24px title, "Add to folder", Share button, grey AI text, bottom **recording button + "Ask anything" bar** with a recipe chip |
| `granola-09-transcript-me-bubbles.png` | **Transcript "Me" bubbles:** right-aligned, solid teal, white text; hover row shows `00:00` + copy + delete |
| `granola-10-app-sidebar-chat.jpg` | **Real app with sidebar:** search ⌘K, Home / Shared with me / Chat, "Spaces" section; chat with right-aligned grey user bubbles, grey agent steps, plain AI answer, "Ask anything" bar with Auto model picker + mic |
| `granola-11-sidebar-zoom.png` | Close-up of the sidebar: row height, soft active fill, section label style |
| `granola-12-brief-agent-steps.jpg` | Agent progress list: checked sub-steps on a vertical guide line, "Preparing your brief…" |
| `granola-13-brief-card-ai-text.jpg` | Brief card typography: bold entities in AI prose, grey bullets |
| `circleback-01-hero.jpg` | Homepage with the embedded app demo |
| `circleback-02-demo-meeting-notes.jpg` | **Live demo app:** 228px sidebar (New meeting, Home, Meetings, Action Items 2, Automations, Insights, Ask; Spaces: People, Companies); meeting header chips (Today · 52m · ● Planning · Add tag); Notes / Transcript underline tabs; action items with assignee chips; Overview bullets; right cards (People with talk-time bars, Recent conversations); bottom "Ask about…" bar |
| `circleback-03-demo-transcript.jpg` | **Circleback transcript:** "Search transcript…" field, avatar + name + right timestamp, paragraph text with no bubbles |
| `circleback-04-ask-bar.jpg` | Floating ask bar over notes with @, attach and send |
| `gumloop-01-hero.jpg` | Gumloop hero: black primary button, white secondary, 8px radius, Geist |
| `gumloop-02-app-mockup.jpg` | App mockup: grouped sidebar (Revenue / Customer Success), agent page, right config panel with grey section headers (Model, Connectors, Skills) |
| `gumloop-03-app-mockup-zoom.png` | Close-up of the above: list density, 8px rounded fields, cool hairlines |
| `gumloop-04-chat-agent-steps.jpg` | Chat: user bubble with avatar, agent name, tool steps with icons, result table |
| `gumloop-05-home-route.jpg` | gumloop.com/home (signed out, it shows the marketing page) |
| `linear-01-hero-dark.jpg` | Linear landing page before load: near-black canvas and nav (restraint) |
| `linear-02-app-mockup-dark.jpg` | Linear dark app mockup: 28px nav rows at 13px, muted section labels, issue title 20/590, activity feed, agent side panel |

**Raw token dumps captured during research** (for traceability; not to be copied as-is):
- Granola `--oats-*` (light + `html.dark`)
- Circleback `--color-*` (`:root` + `.dark`)
- Circleback demo metrics: nav 13/500 at 31px r7, body 15/450 at 28px line-height, cards r12 with 1px `#EFEFEF`
- Gumloop: Geist 14/500, labels 12/500, r8, 1px ≈ `#E8E8EC`
- Linear: Inter Variable 13/510 nav, 28px rows r8, 12/510 `#62666D` labels, body 14/21 `#8A8F98`, bg `#08090A`
