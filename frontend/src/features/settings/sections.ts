/**
 * The settings pages (ui.md §3 Settings layout), registered from each
 * feature's own `settings-page.tsx` (REG-02): a feature that adds a page
 * exports `settingsPages` there and declares its id in `SettingsPageIds`
 * (`./search`); nothing here lists pages. The nav shows them by group, then
 * by `order`.
 */
import type { ComponentType } from 'react';
import { z } from 'zod';

import type { Icon } from '@/ui/icons';

import { settingsCopy } from './copy';
import type { SettingsSectionId } from './search';

/** A nav group. */
export type SettingsGroup = keyof typeof settingsCopy.groups;

/** The nav groups, in order. */
export const SETTINGS_GROUPS = [
  'app',
  'capture',
  'ai',
  'workspace',
  'privacy',
  'data',
  'more',
] as const satisfies readonly SettingsGroup[];

/** One settings page, as a feature's `settings-page.tsx` registers it. */
export interface SettingsPage {
  id: SettingsSectionId;
  group: SettingsGroup;
  /** Sort key within its group; leave gaps of 10 for later pages. */
  order: number;
  icon: Icon;
  /** The nav entry and the page's heading. */
  label: string;
  /** One line under the heading. */
  description: string;
  Component: ComponentType;
}

/** React components and Lucide icons are functions or `forwardRef`/`memo` objects. */
function isComponent(value: unknown): boolean {
  return typeof value === 'function' || (typeof value === 'object' && value !== null);
}

/**
 * A page as a module exports it. Its id is a string at runtime: the set of
 * registered pages is what defines `SettingsSectionId`, and each page file
 * checks its own entry with `satisfies readonly SettingsPage[]`.
 */
const pageSchema = z.object({
  id: z.custom<SettingsSectionId>((value) => typeof value === 'string' && value !== ''),
  group: z.enum(SETTINGS_GROUPS),
  order: z.number().finite(),
  icon: z.custom<Icon>(isComponent),
  label: z.string().min(1),
  description: z.string().min(1),
  Component: z.custom<ComponentType>(isComponent),
});

const pageModuleSchema = z.object({ settingsPages: z.array(pageSchema) });

/** Every `features/<feature>/settings-page.tsx`, loaded with the settings screen. */
const pageModules = import.meta.glob<unknown>('../*/settings-page.{ts,tsx}', { eager: true });

/** Group order, then `order`, then id: the same order on every build. */
function byPlace(a: SettingsPage, b: SettingsPage): number {
  const group = SETTINGS_GROUPS.indexOf(a.group) - SETTINGS_GROUPS.indexOf(b.group);
  return group || a.order - b.order || a.id.localeCompare(b.id);
}

/** The registered pages, checked and sorted. A malformed page module is a bug, so it throws. */
function collectPages(): readonly SettingsPage[] {
  const pages = Object.entries(pageModules).flatMap(([path, module]) => {
    const parsed = pageModuleSchema.safeParse(module);
    if (!parsed.success) {
      throw new Error(`${path} must export settingsPages: ${parsed.error.message}`);
    }
    return parsed.data.settingsPages;
  });
  return pages.sort(byPlace);
}

let registered: readonly SettingsPage[] | undefined;

/**
 * Every page, in nav order. Read on first use, not at import: page modules
 * import features that import this one, so the list is complete only once
 * every module has loaded.
 */
export function settingsSections(): readonly SettingsPage[] {
  registered ??= collectPages();
  return registered;
}

/** Whether `value` names a registered page. */
export function isSettingsSectionId(value: unknown): value is SettingsSectionId {
  return settingsSections().some((page) => page.id === value);
}
