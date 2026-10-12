/**
 * The settings registry (ui.md §3 Settings layout, UX-19): a few host
 * pages, and the sections features place on them. Both register from each
 * feature's own `settings-page.tsx` (REG-02): a module exports
 * `settingsPages` (host pages) and/or `settingsSections` (sections, each
 * naming its host `page`), and declares every id it adds in
 * `SettingsPageIds` (`./search`); nothing here lists pages or sections.
 *
 * Every id is a link target: `/settings?section=<id>` opens a page at its
 * top, or a section's host page scrolled to that section. A section's
 * `aliases` keep the ids of pages it replaced working, so old links land in
 * the right place.
 */
import type { ComponentType } from 'react';
import { z } from 'zod';

import type { Icon } from '@/ui/icons';

import type { SettingsSectionId } from './search';

/** One host page, as a `settings-page.tsx` registers it. */
export interface SettingsPage {
  id: SettingsSectionId;
  /** Nav order; leave gaps of 10 for later pages. */
  order: number;
  icon: Icon;
  /** The nav entry and the page's heading. */
  label: string;
  /** One line under the heading. */
  description: string;
}

/** One section of a host page, as a `settings-page.tsx` registers it. */
export interface SettingsPageSection {
  /** Its own link target; may equal its host page's id, never another page's. */
  id: SettingsSectionId;
  /** The host page it sits on. */
  page: SettingsSectionId;
  /** Order on the page; leave gaps of 10 for later sections. */
  order: number;
  /** Ids of retired pages or sections that now open this one. */
  aliases?: readonly SettingsSectionId[] | undefined;
  Component: ComponentType;
}

/** Where a link target lands: a page, scrolled to a section when it names one. */
export interface SettingsTarget {
  page: SettingsPage;
  /** The section to scroll to; `undefined` for the page's top. */
  section: SettingsSectionId | undefined;
}

/** React components and Lucide icons are functions or `forwardRef`/`memo` objects. */
function isComponent(value: unknown): boolean {
  return typeof value === 'function' || (typeof value === 'object' && value !== null);
}

/**
 * An id is a string at runtime: the registered pages and sections define
 * `SettingsSectionId`, and each module checks its own entries with
 * `satisfies readonly SettingsPage[]` or `SettingsPageSection[]`.
 */
const idSchema = z.custom<SettingsSectionId>((value) => typeof value === 'string' && value !== '');

const pageSchema = z.object({
  id: idSchema,
  order: z.number().finite(),
  icon: z.custom<Icon>(isComponent),
  label: z.string().min(1),
  description: z.string().min(1),
});

const sectionSchema = z.object({
  id: idSchema,
  page: idSchema,
  order: z.number().finite(),
  aliases: z.array(idSchema).readonly().optional(),
  Component: z.custom<ComponentType>(isComponent),
});

const moduleSchema = z.object({
  settingsPages: z.array(pageSchema).optional(),
  settingsSections: z.array(sectionSchema).optional(),
});

/** Every `features/<feature>/settings-page.tsx`, loaded with the settings screen. */
const pageModules = import.meta.glob<unknown>('../*/settings-page.{ts,tsx}', { eager: true });

/** The registry, checked and sorted. */
export interface SettingsRegistry {
  pages: readonly SettingsPage[];
  sections: readonly SettingsPageSection[];
}

/** `order`, then id: the same order on every build. */
function byOrder(a: { order: number; id: string }, b: { order: number; id: string }): number {
  return a.order - b.order || a.id.localeCompare(b.id);
}

/** Throws unless every id is unique and every section sits on a registered page. */
export function checkRegistry({ pages, sections }: SettingsRegistry): void {
  const pageIds = new Set(pages.map((page) => page.id));
  const seen = new Set<string>(pageIds);
  if (seen.size !== pages.length) throw new Error('two settings pages share an id');
  const sectionIds = new Set<string>();
  for (const section of sections) {
    if (!pageIds.has(section.page)) {
      throw new Error(`settings section ${section.id} names an unknown page ${section.page}`);
    }
    if (sectionIds.has(section.id))
      throw new Error(`settings id ${section.id} is registered twice`);
    sectionIds.add(section.id);
    const ownPage = section.id === section.page;
    for (const id of [...(ownPage ? [] : [section.id]), ...(section.aliases ?? [])]) {
      if (seen.has(id)) throw new Error(`settings id ${id} is registered twice`);
      seen.add(id);
    }
  }
}

/** The registered pages and sections, checked and sorted. A malformed module is a bug, so it throws. */
export function collectRegistry(modules: Record<string, unknown>): SettingsRegistry {
  const pages: SettingsPage[] = [];
  const sections: SettingsPageSection[] = [];
  for (const [path, module] of Object.entries(modules)) {
    const parsed = moduleSchema.safeParse(module);
    if (!parsed.success) {
      throw new Error(
        `${path} must export settingsPages or settingsSections: ${parsed.error.message}`,
      );
    }
    pages.push(...(parsed.data.settingsPages ?? []));
    sections.push(...(parsed.data.settingsSections ?? []));
  }
  const registry = { pages: pages.sort(byOrder), sections: sections.sort(byOrder) };
  checkRegistry(registry);
  return registry;
}

let registered: SettingsRegistry | undefined;

/**
 * The registry. Read on first use, not at import: page modules import
 * features that import this one, so it is complete only once every module
 * has loaded.
 */
function registry(): SettingsRegistry {
  registered ??= collectRegistry(pageModules);
  return registered;
}

/** Every host page, in nav order. */
export function settingsPages(): readonly SettingsPage[] {
  return registry().pages;
}

/** The sections on page `pageId`, in order. */
export function sectionsOf(pageId: SettingsSectionId): readonly SettingsPageSection[] {
  return registry().sections.filter((section) => section.page === pageId);
}

/** Where `id` lands in `registry`: its page, or its (or its alias's) section's page. */
export function resolveIn(
  { pages, sections }: SettingsRegistry,
  id: string,
): SettingsTarget | undefined {
  const page = pages.find((candidate) => candidate.id === id);
  if (page !== undefined) return { page, section: undefined };
  const section = sections.find(
    (candidate) => candidate.id === id || (candidate.aliases ?? []).some((alias) => alias === id),
  );
  const host = pages.find((candidate) => candidate.id === section?.page);
  return section === undefined || host === undefined
    ? undefined
    : { page: host, section: section.id };
}

/** Where a link to `id` lands, or `undefined` for an unknown id. */
export function resolveSettingsTarget(id: string): SettingsTarget | undefined {
  return resolveIn(registry(), id);
}

/** Whether `value` names a registered page, section or alias. */
export function isSettingsSectionId(value: unknown): value is SettingsSectionId {
  return typeof value === 'string' && resolveSettingsTarget(value) !== undefined;
}
