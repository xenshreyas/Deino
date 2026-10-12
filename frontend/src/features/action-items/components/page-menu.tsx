/**
 * The page's ⋯ menu: "Mark all mine done" (Circleback §8) and the 9:00
 * reminder switch, which is `notifications.preferences.showActionItemReminders`
 * in the settings document.
 */

import { usePatchSettings, useSettings } from '@/features/settings';
import { Bell, Check, CheckCheck, Ellipsis, ICON_PROPS } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { copy } from '../copy';
import { useMarkAllMineDone } from '../queries';

/** The reminder's current switch and a toggle. */
function useReminderSwitch() {
  const settings = useSettings();
  const patch = usePatchSettings();
  const on = settings.data?.notifications.preferences.showActionItemReminders ?? false;
  const toggle = () => {
    patch.mutate({ notifications: { preferences: { showActionItemReminders: !on } } });
  };
  return { on, toggle, ready: settings.isSuccess };
}

/** The ⋯ button and its menu. */
export function PageMenu() {
  const markAll = useMarkAllMineDone();
  const reminder = useReminderSwitch();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <IconButton icon={Ellipsis} label={copy.menu.label} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-[240px]">
        <DropdownMenuItem
          onSelect={() => {
            // The list empties on screen (UX-32: no toast).
            markAll.mutate(undefined);
          }}
        >
          <CheckCheck {...ICON_PROPS} />
          {copy.menu.markAllMineDone}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          disabled={!reminder.ready}
          onSelect={reminder.toggle}
          className="h-auto py-1.5"
        >
          <Bell {...ICON_PROPS} />
          <span className="flex flex-1 flex-col">
            {copy.menu.reminder}
            <span className="text-xs text-fg-muted">{copy.menu.reminderHint}</span>
          </span>
          {reminder.on ? <Check {...ICON_PROPS} className="text-accent" /> : null}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
