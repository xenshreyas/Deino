/**
 * The record toggle on a Coming up row: a filled dot when the event will
 * record on its own, a ring when it will not, with the reason as its hint.
 * A one-off event flips at once; a recurring one asks "Just this one" or
 * "All occurrences" (Circleback §3.2), and can go back to the settings.
 */
import { toast } from 'sonner';

import type { CalendarEvent, RecordChoice, RecordDecision } from '@/features/calendar';
import { useChooseRecord } from '@/features/calendar';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Hint } from '@/ui/tooltip';

import { homeCopy } from '../copy';
import { flipChoice, isRecurring } from '../event-actions';

/** Props of {@link RecordToggle}. */
export interface RecordToggleProps {
  event: CalendarEvent;
  decision: RecordDecision;
}

/** The dot itself; a button so it can trigger a menu. */
function Dot({ on, ...props }: { on: boolean } & React.ComponentProps<typeof Button>) {
  return (
    <Button variant="ghost" size="icon" aria-pressed={on} {...props}>
      <span
        aria-hidden
        className={cn(
          'size-2.5 rounded-full border transition-colors duration-150',
          on ? 'border-accent bg-accent' : 'border-fg-subtle bg-transparent',
        )}
      />
    </Button>
  );
}

/** Props of the recurring event's menu. */
interface RecurringMenuProps {
  decision: RecordDecision;
  label: string;
  apply: (choice: RecordChoice) => void;
}

/** One choice in the menu. */
function Choice({
  choice,
  apply,
  children,
}: { choice: RecordChoice } & Pick<RecurringMenuProps, 'apply'> & { children: string }) {
  return (
    <DropdownMenuItem
      onSelect={() => {
        apply(choice);
      }}
    >
      {children}
    </DropdownMenuItem>
  );
}

/** "Just this one", "All occurrences" or back to the settings. */
function RecurringMenu({ decision, label, apply }: RecurringMenuProps) {
  return (
    <DropdownMenu>
      <Hint label={label}>
        <DropdownMenuTrigger asChild>
          <Dot on={decision.record} aria-label={label} />
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel>
          {decision.record ? homeCopy.neverRecord : homeCopy.alwaysRecord}
        </DropdownMenuLabel>
        <Choice choice={flipChoice(decision, 'thisEvent')} apply={apply}>
          {homeCopy.justThisOne}
        </Choice>
        <Choice choice={flipChoice(decision, 'allOccurrences')} apply={apply}>
          {homeCopy.allOccurrences}
        </Choice>
        <DropdownMenuSeparator />
        <Choice choice={{ record: null, scope: 'allOccurrences' }} apply={apply}>
          {homeCopy.followRules}
        </Choice>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The record toggle. */
export function RecordToggle({ event, decision }: RecordToggleProps) {
  const choose = useChooseRecord();
  const label = `${decision.record ? homeCopy.records : homeCopy.skips}. ${homeCopy.reasons[decision.reason]}`;
  const apply = (choice: RecordChoice) => {
    choose.mutate({ id: event.id, choice }, { onError: () => toast.error(homeCopy.toggleFailed) });
  };
  if (isRecurring(event)) return <RecurringMenu decision={decision} label={label} apply={apply} />;
  return (
    <Hint label={label}>
      <Dot
        on={decision.record}
        aria-label={label}
        onClick={() => {
          apply(flipChoice(decision, 'thisEvent'));
        }}
      />
    </Hint>
  );
}
