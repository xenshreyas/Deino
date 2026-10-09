# 0003. Build the UI with Vite, TanStack Router, Tailwind v4 and Vitest

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-10a (decided in PARITY_PLAN §4.0 from `docs/references/architecture.md` §7)

## Context

The UI was a Next.js 14 app built with `output: 'export'` and loaded by Tauri
from static files. Under static export almost none of Next's features apply:
no server rendering, server components, image optimisation or route handlers.
What remained had costs:

- Dynamic segments need `generateStaticParams`, so the meeting page lived at
  `/meeting-details?id=` with untyped search params read through
  `useSearchParams` (and a `Suspense` boundary Next required for it).
- BlockNote and TipTap only worked after a webpack alias block in
  `next.config.js` forced single copies of the ProseMirror packages.
- Tests ran on `bun:test` with `mock.module`, outside the build pipeline, so
  they couldn't share its aliases and had no coverage tooling.
- None of the reference apps (anarlog, Handy, spacedrive, Cap) use Next.js, so
  their routing, testing and Tauri patterns couldn't be copied directly.

## Decision

- **Vite 8** with `@vitejs/plugin-react` builds `index.html` + `src/main.tsx`
  into `frontend/dist/` (Tauri `frontendDist`). The dev server keeps port 3118
  (`devUrl`), with `strictPort` so Tauri never attaches to a stale server.
- **TanStack Router**, file-based, through `@tanstack/router-plugin` with
  `autoCodeSplitting`. Routes live in `src/routes/` and stay thin: each renders
  one screen from `src/features/<feature>/`. The plugin generates
  `src/routeTree.gen.ts` on every dev and build run; it is committed so
  `tsc` and editors work on a fresh checkout.
  - `/` record, `/settings`, `/meetings/$meetingId` (typed path param, zod
    search schema for `?source=recording`).
  - `/meeting-details?id=` remains as a redirect route so old URLs keep working.
  - The root route renders the app shell (`features/shell/`): providers,
    onboarding gate, sidebar, drag-and-drop import and the tray listener.
- **Tailwind v4** through `@tailwindcss/vite`, configured in
  `src/styles/globals.css` (`@theme`, `@plugin`). The v3 config was ported with
  `@tailwindcss/upgrade`, plus compatibility rules for v3 defaults that v4
  changed (border colour, button cursor, placeholder colour), so the UI looks
  the same. PostCSS, autoprefixer, `tailwindcss-animate` (now `tw-animate-css`)
  and `@tailwindcss/container-queries` (built in) are gone.
- **Fonts** come from `@fontsource/source-sans-3` instead of `next/font`.
- **ProseMirror** stays single-instance through the existing `pnpm.overrides`
  pins; no alias or `resolve.dedupe` is needed.
- **Vitest 5** with happy-dom and Testing Library, configured in the `test`
  block of `vite.config.ts` so tests share the app's aliases. Tests are
  colocated as `*.test.ts(x)`. `src/test-setup.ts` installs a minimal
  `__TAURI_INTERNALS__` shim adapted from anarlog (MIT).
- **ESLint 9** flat config (`eslint.config.mjs`) with typescript-eslint and
  react-hooks. Legacy-heavy rules are warnings until FND-13 makes them errors.

## Alternatives

- **Stay on Next.js static export:** no migration cost, but keeps every problem
  above and diverges further from the references as the app grows.
- **React Router 7 (spacedrive):** mature, but its typed routes and search-param
  validation are weaker, and anarlog (the closest product analog) uses TanStack
  Router with the same file-based layout we want.
- **Hand-rolled routing (Handy):** fine for a settings window, not for a
  multi-screen app with deep links and typed params.
- **jsdom instead of happy-dom:** anarlog uses jsdom; happy-dom is faster and
  sufficient for our hook and component tests.

## Consequences

- Route params and search params are typed end to end; navigation uses
  `useNavigate({ to, params, search })` instead of string URLs.
- `pnpm dev`, `pnpm build`, `pnpm test`, `pnpm typecheck` and `pnpm lint` are the
  standard checks; `bun` is no longer needed.
- `routeTree.gen.ts` changes whenever a route file is added or removed; it is
  generated, ignored by ESLint, and must be committed with the route.
- Test files inside `src/routes/` need a `-` prefix so the router plugin skips
  them.
- React StrictMode stays off, matching the old `reactStrictMode: false`, until
  NOT-01 replaces BlockNote.
- `pnpm dev:mock` (FND-04) can now be a Vite mode with an alias swap for the
  generated IPC bindings.
