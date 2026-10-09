import { useNavigate } from '@tanstack/react-router';

import { CommandGroup, CommandItem } from '@/ui/command';
import { ICON_PROPS } from '@/ui/icons';

import { NAV_ITEMS } from '../nav';
import { paletteCopy } from './copy';
import type { PaletteProviderProps } from './registry';

/** "Go to": every sidebar destination, straight from the nav registry. */
export function GoToProvider({ close }: PaletteProviderProps) {
  const navigate = useNavigate();
  return (
    <CommandGroup heading={paletteCopy.goTo}>
      {NAV_ITEMS.map((item) => {
        const ItemIcon = item.icon;
        return (
          <CommandItem
            key={item.id}
            value={`go-to ${item.label}`}
            keywords={[item.label]}
            onSelect={() => {
              close();
              void navigate({ to: item.to });
            }}
          >
            <ItemIcon {...ICON_PROPS} />
            {item.label}
          </CommandItem>
        );
      })}
    </CommandGroup>
  );
}
