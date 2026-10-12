/**
 * The Automations page's list: a quiet rail of names, marked "Off" or
 * "Manual" when they do not run on their own. While a new automation is
 * being created, its draft heads the rail as the current entry.
 */
import { cn } from '@/ui/cn';

import { automationsCopy } from '../copy';
import type { Automation, AutomationId } from '../types';

/** Props of {@link AutomationRail}. */
export interface AutomationRailProps {
  automations: readonly Automation[];
  /** The automation shown, or `null` while creating one. */
  selected: AutomationId | null;
  /** True while the builder shows a new, unsaved automation. */
  creating: boolean;
  onSelect: (id: AutomationId) => void;
}

/** "Off", "Manual" or nothing. */
function modeOf(automation: Automation): string | null {
  if (!automation.enabled) return automationsCopy.off;
  return automation.autoRun ? null : automationsCopy.manualOnly;
}

/** One button per automation, after the draft when there is one. */
export function AutomationRail({ automations, selected, creating, onSelect }: AutomationRailProps) {
  return (
    <nav aria-label={automationsCopy.title}>
      <ul className="grid gap-0.5">
        {creating ? (
          <li>
            <RailEntry
              name={automationsCopy.untitled}
              note={automationsCopy.draft}
              active
              onClick={() => undefined}
            />
          </li>
        ) : null}
        {automations.map((automation) => (
          <li key={automation.id}>
            <RailEntry
              name={automation.name}
              note={modeOf(automation)}
              active={!creating && automation.id === selected}
              onClick={() => {
                onSelect(automation.id);
              }}
            />
          </li>
        ))}
      </ul>
    </nav>
  );
}

/** A 30px nav row: the name, truncated, and a quiet note on the right. */
function RailEntry(props: {
  name: string;
  note: string | null;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-current={props.active ? 'page' : undefined}
      onClick={props.onClick}
      className={cn(
        'flex h-[30px] w-full items-center gap-2 rounded-md px-2 text-left text-sm',
        'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg',
        'focus-visible:outline-2 focus-visible:outline-focus-ring',
        props.active ? 'bg-bg-muted font-semibold text-fg' : 'text-fg-muted',
      )}
    >
      <span className="min-w-0 flex-1 truncate">{props.name}</span>
      {props.note === null ? null : (
        <span className="shrink-0 text-xs font-normal text-fg-subtle">{props.note}</span>
      )}
    </button>
  );
}
