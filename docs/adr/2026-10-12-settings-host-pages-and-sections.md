# Settings: host pages, and sections features place on them

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** UX-19 (docs/UX_SIMPLIFICATION.md); amends
  [REG-02](2026-10-12-settings-self-registration.md) decision 2

## Context

Settings had 27 pages in 7 nav groups: every feature registered a page of
its own from its `settings-page.ts(x)`. The UX simplification plan targets 9
pages with no group headings, where most features contribute a section to a
shared page. Links into settings (`/settings?section=<id>`, in-app links,
e2e tests) name the old page ids and must keep working. REG-02's rule stays:
no central list of pages or sections.

## Decision

1. A `settings-page.ts(x)` exports `settingsPages` (host pages: `id`,
   `order`, `icon`, `label`, `description`; no `group`, no component)
   and/or `settingsSections` (`id`, `page`, `order`, `Component`, optional
   `aliases`). The settings feature registers the nine host pages and its
   own sections; every other feature registers sections only.
2. `sections.ts` collects both with `import.meta.glob`, checks them with zod
   and `checkRegistry` (every section names a registered page; page,
   section and alias ids are unique, except that a section may share its
   own page's id) and sorts by `order`, then id.
3. Every id is a link target in `SettingsPageIds`: a page opens at its top;
   a section (or an alias, the id of a page it replaced) opens its host page
   scrolled to the section (`resolveSettingsTarget`, anchors
   `settings-section-<id>`). `/settings?section=` keeps its name and schema.
4. The nav lists the pages only, in `order`; `SETTINGS_GROUPS` and the
   group copy are gone.

## Consequences

- A new setting usually joins an existing page as a section; a new host
  page is a deliberate IA change in `features/settings/settings-page.tsx`.
- Old ids never break: retire a section by moving its id to another
  section's `aliases`.
- Sections render their own `SettingsSection` heading; the page header is
  the page's label and description.
