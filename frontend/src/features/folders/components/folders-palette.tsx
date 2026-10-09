/**
 * ⌘K results for folders: open any folder by name, with its path.
 * Registered in the shell's `command-palette/providers.ts`.
 */
import { useNavigate } from '@tanstack/react-router';

import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import { ICON_PROPS } from '@/ui/icons';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import { useFolderTree } from '../queries';
import { folderPath } from '../tree';

/** What every palette provider receives (structurally the shell's props). */
export interface FoldersPaletteProps {
  search: string;
  close: () => void;
}

/** "Folders": every folder, shown once the user types. */
export function FoldersPalette({ search, close }: FoldersPaletteProps) {
  const navigate = useNavigate();
  const entries = useFolderTree().data?.entries ?? [];
  if (search.trim() === '' || entries.length === 0) return null;
  return (
    <CommandGroup heading={foldersCopy.palette}>
      {entries.map(({ folder }) => {
        const ItemIcon = folderIcon(folder.icon);
        const parents = folderPath(entries, folder.id).slice(0, -1);
        return (
          <CommandItem
            key={folder.id}
            value={`folder ${folder.id} ${folder.name}`}
            keywords={[folder.name, ...parents]}
            onSelect={() => {
              close();
              void navigate({ to: '/folders/$folderId', params: { folderId: folder.id } });
            }}
          >
            <ItemIcon {...ICON_PROPS} />
            <span className="truncate">{folder.name}</span>
            <CommandShortcut>
              {parents.length === 0 ? foldersCopy.palette : parents.join(' / ')}
            </CommandShortcut>
          </CommandItem>
        );
      })}
    </CommandGroup>
  );
}
