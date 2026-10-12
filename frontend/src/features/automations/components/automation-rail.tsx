/**
 * The Automations page's list: a quiet rail of names, marked "Off" or
 * "Manual" when they do not run on their own.
 */
import { cn } from '@/ui/cn';

import { automationsCopy } from '../copy';
import type { Automation, AutomationId } from '../types';

/** Props of {@link AutomationRail}. */
export interface AutomationRailProps {
  automations: readonly Automation[];
  selected: AutomationId | null;
  onSelect: (id: AutomationId) => void;
}

/** "Off", "Manual" or nothing. */
function modeOf(automation: Automation): string | null {
  if (!automation.enabled) return automationsCopy.off;
  return automation.autoRun ? null : automationsCopy.manualOnly;
}

/** One button per automation. */
export function AutomationRail({ automations, selected, onSelect }: AutomationRailProps) {
  return (
    <nav aria-label={automationsCopy.title}>
      <ul className="grid gap-0.5">
        {automations.map((automation) => {
          const active = automation.id === selected;
          const mode = modeOf(automation);
          return (
            <li key={automation.id}>
              <button
                type="button"
                aria-current={active ? 'page' : undefined}
                onClick={() => {
                  onSelect(automation.id);
                }}
                className={cn(
                  'flex h-[30px] w-full items-center gap-2 rounded-md px-2 text-left text-sm',
                  'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg',
                  'focus-visible:outline-2 focus-visible:outline-focus-ring',
                  active ? 'bg-bg-muted font-semibold text-fg' : 'text-fg-muted',
                )}
              >
                <span className="min-w-0 flex-1 truncate">{automation.name}</span>
                {mode === null ? null : (
                  <span className="shrink-0 text-xs font-normal text-fg-subtle">{mode}</span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
