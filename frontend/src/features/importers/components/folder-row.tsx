import { useState } from 'react';

import { Button } from '@/ui/button';

import { importCopy, sourceCopy } from '../copy';
import { usePickImportFolder } from '../queries';
import { PreviewActions } from './preview-actions';

/** A folder of exported meetings: choose it, then Check and Import. */
export function FolderRow({ running }: { running: boolean }) {
  const [folder, setFolder] = useState<string | null>(null);
  const pick = usePickImportFolder();
  return (
    <div className="flex flex-col gap-3 px-4 py-3 sm:flex-row sm:items-start sm:justify-between">
      <div className="min-w-0">
        <p className="text-sm text-fg">{sourceCopy.folder.name}</p>
        <p className="mt-0.5 text-xs text-fg-muted">{sourceCopy.folder.hint}</p>
        <p className="mt-1 truncate font-mono text-xs text-fg-subtle" title={folder ?? undefined}>
          {folder ?? importCopy.noFolder}
        </p>
        <Button
          variant="link"
          size="sm"
          className="mt-1 h-auto px-0 text-xs"
          disabled={pick.isPending || running}
          onClick={() => {
            pick.mutate(undefined, {
              onSuccess: (chosen) => {
                if (chosen !== null) setFolder(chosen);
              },
            });
          }}
        >
          {importCopy.chooseFolder}
        </Button>
      </div>
      {/* Remount on a new folder, so its old count goes away. */}
      <PreviewActions
        key={folder ?? 'none'}
        target={folder === null ? null : { source: 'folder', folder }}
        running={running}
      />
    </div>
  );
}
