/**
 * The settings screen (ui.md §3 Settings layout, §4.6): a 200px section nav
 * beside a 720px content column. The open page lives in the URL
 * (`?section=`), so links and the palette can open any page directly.
 */
import { Link } from '@tanstack/react-router';

import { cn } from '@/ui/cn';
import { ICON_PROPS } from '@/ui/icons';

import { settingsCopy } from './copy';
import type { SettingsSectionId } from './search';
import { SETTINGS_SECTIONS, type SettingsGroup } from './sections';

/** Nav groups, in order. */
const GROUPS: readonly SettingsGroup[] = [
  'app',
  'capture',
  'ai',
  'workspace',
  'privacy',
  'data',
  'more',
];

/** The section nav: grouped links, the open one highlighted. */
function SettingsNav({ active }: { active: SettingsSectionId }) {
  return (
    <nav
      aria-label={settingsCopy.navLabel}
      className="flex w-[200px] shrink-0 flex-col gap-px overflow-y-auto px-2 pb-6"
    >
      <h1 className="px-2 pt-2 pb-1 text-base font-semibold text-fg">{settingsCopy.title}</h1>
      {GROUPS.map((group) => (
        <div key={group} className="flex flex-col gap-px">
          <h2 className="mt-3 flex h-6 items-center px-2 text-xs font-semibold text-fg-muted">
            {settingsCopy.groups[group]}
          </h2>
          {SETTINGS_SECTIONS.filter((section) => section.group === group).map((section) => {
            const SectionIcon = section.icon;
            return (
              <Link
                key={section.id}
                to="/settings"
                search={{ section: section.id }}
                aria-current={section.id === active ? 'page' : undefined}
                className={cn(
                  'flex h-[30px] items-center gap-2 rounded-sm px-2 text-sm text-fg-muted transition-colors duration-(--dur-fast)',
                  'hover:bg-bg-muted hover:text-fg aria-[current=page]:bg-bg-muted aria-[current=page]:text-fg',
                )}
              >
                <SectionIcon {...ICON_PROPS} />
                {settingsCopy.sections[section.id].label}
              </Link>
            );
          })}
        </div>
      ))}
    </nav>
  );
}

/** Props of {@link SettingsScreen}. */
export interface SettingsScreenProps {
  section: SettingsSectionId;
}

/** The settings screen with one page open. */
export function SettingsScreen({ section }: SettingsScreenProps) {
  const page =
    SETTINGS_SECTIONS.find((candidate) => candidate.id === section) ?? SETTINGS_SECTIONS[0];
  if (page === undefined) return null;
  const { label, description } = settingsCopy.sections[page.id];
  const PageComponent = page.Component;
  return (
    <div className="flex h-full">
      <SettingsNav active={page.id} />
      <div className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-[720px] px-6 pt-2 pb-16">
          <header className="pb-6">
            <h2 className="text-xl font-semibold text-fg">{label}</h2>
            <p className="mt-1 text-sm text-fg-muted">{description}</p>
          </header>
          <PageComponent key={page.id} />
        </div>
      </div>
    </div>
  );
}
