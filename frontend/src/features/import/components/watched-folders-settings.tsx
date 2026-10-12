/**
 * Settings → Watched folders (CAP-11): the folders whose new recordings
 * import on their own, each with "Move to processed", and the files that
 * could not be imported, with Retry and Dismiss.
 */
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { FolderPlus, ICON_PROPS } from '@/ui/icons';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { importCopy as copy } from '../copy';
import { useAddWatchedFolder, useWatchedFolders, useWatchFailures } from '../queries';
import { FailureList } from './failure-list';
import { FolderList } from './folder-list';

/** Settings → Watched folders. */
export function WatchedFoldersSettings() {
  return (
    <div className="flex flex-col gap-8">
      <p className="max-w-prose text-sm text-pretty text-fg-muted">{copy.intro}</p>
      <FoldersSection />
      <FailuresSection />
    </div>
  );
}

/** The folder list, or its loading or error state, and Add folder. */
function FoldersSection() {
  const folders = useWatchedFolders();
  const add = useAddWatchedFolder();
  const addFolder = () => {
    add.mutate(undefined, {
      onError: (error) => toast.error(copy.addFailed, { description: error.message }),
    });
  };
  return (
    <SettingsSection title={copy.foldersTitle} description={copy.foldersDescription}>
      {folders.isPending ? <SkeletonText lines={2} /> : null}
      {folders.isError ? <p className="text-sm text-danger">{copy.loadError}</p> : null}
      {folders.data === undefined ? null : <FolderList folders={folders.data} />}
      <div>
        <Button variant="secondary" size="sm" disabled={add.isPending} onClick={addFolder}>
          <FolderPlus {...ICON_PROPS} />
          {copy.addFolder}
        </Button>
      </div>
    </SettingsSection>
  );
}

/** The failure list, once loaded. */
function FailuresSection() {
  const failures = useWatchFailures();
  if (failures.data === undefined) return null;
  return (
    <SettingsSection title={copy.failuresTitle} description={copy.failuresDescription}>
      <FailureList failures={failures.data} />
    </SettingsSection>
  );
}
