/**
 * The settings screen (ui.md §3 Settings layout, §4.6, UX-19): a 200px nav
 * of the host pages, no group headings, beside a 720px column of the open
 * page's sections. The open page lives in the URL (`?section=`), so links
 * and the palette can open any page or section directly; a section's id
 * (or a retired page's id) opens its page scrolled to it.
 */
import { Link } from '@tanstack/react-router';

import { useMountEffect } from '@/hooks/useMountEffect';
import { cn } from '@/ui/cn';
import { ICON_PROPS } from '@/ui/icons';

import { settingsCopy } from './copy';
import type { SettingsSectionId } from './search';
import {
  resolveSettingsTarget,
  sectionsOf,
  type SettingsPage,
  settingsPages,
  type SettingsTarget,
} from './sections';

/** The DOM id of a section's anchor. */
export function sectionAnchor(id: SettingsSectionId): string {
  return `settings-section-${id}`;
}

/** The page nav: one link per host page, the open one highlighted. */
function SettingsNav({ active }: { active: SettingsSectionId }) {
  return (
    <nav
      aria-label={settingsCopy.navLabel}
      className="flex w-[200px] shrink-0 flex-col gap-px overflow-y-auto px-2 pb-6"
    >
      <h1 className="px-2 pt-2 pb-3 text-base font-semibold text-fg">{settingsCopy.title}</h1>
      {settingsPages().map((page) => {
        const PageIcon = page.icon;
        return (
          <Link
            key={page.id}
            to="/settings"
            search={{ section: page.id }}
            aria-current={page.id === active ? 'page' : undefined}
            className={cn(
              'flex h-[30px] items-center gap-2 rounded-sm px-2 text-sm text-fg-muted transition-colors duration-(--dur-fast)',
              'hover:bg-bg-muted hover:text-fg aria-[current=page]:bg-bg-muted aria-[current=page]:text-fg',
            )}
          >
            <PageIcon {...ICON_PROPS} />
            {page.label}
          </Link>
        );
      })}
    </nav>
  );
}

/** The open page's sections, scrolled to `section` once mounted. */
function PageSections({ page, section }: SettingsTarget) {
  useMountEffect(() => {
    if (section === undefined) return;
    document.getElementById(sectionAnchor(section))?.scrollIntoView({ block: 'start' });
  });
  return (
    <div className="flex flex-col gap-10">
      {sectionsOf(page.id).map(({ id, Component }) => (
        <div key={id} id={sectionAnchor(id)} className="scroll-mt-4">
          <Component />
        </div>
      ))}
    </div>
  );
}

/** The first page, for an id that names nothing. */
function fallback(): SettingsTarget | undefined {
  const first: SettingsPage | undefined = settingsPages()[0];
  return first === undefined ? undefined : { page: first, section: undefined };
}

/** Props of {@link SettingsScreen}. */
export interface SettingsScreenProps {
  section: SettingsSectionId;
}

/** The settings screen with one page open. */
export function SettingsScreen({ section }: SettingsScreenProps) {
  const target = resolveSettingsTarget(section) ?? fallback();
  if (target === undefined) return null;
  const { page } = target;
  return (
    <div className="flex h-full">
      <SettingsNav active={page.id} />
      <div className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-[720px] px-6 pt-2 pb-16">
          <header className="pb-6">
            <h2 className="text-xl font-semibold text-fg">{page.label}</h2>
            <p className="mt-1 text-sm text-fg-muted">{page.description}</p>
          </header>
          <PageSections key={section} page={page} section={target.section} />
        </div>
      </div>
    </div>
  );
}
