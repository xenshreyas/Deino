import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { Skeleton } from '@/ui/skeleton';

import { importCopy } from '../copy';
import { useImportProgress, useImportSources } from '../queries';
import { FolderRow } from './folder-row';
import { ImportStatus, isRunning } from './import-status';
import { ProviderRow } from './provider-row';

/** Settings → Import: the job, the note-takers and a folder (container). */
export function ImportSettings() {
  const sources = useImportSources();
  const progress = useImportProgress();
  const running = isRunning(progress.data);
  return (
    <div className="flex flex-col gap-8">
      {progress.data === undefined || progress.data.phase === 'idle' ? null : (
        <ImportStatus progress={progress.data} />
      )}
      <SettingsSection title={importCopy.title} description={importCopy.description}>
        {sources.isPending ? <Skeleton className="h-48 w-full" /> : null}
        {sources.isError ? (
          <p role="alert" className="text-sm text-danger">
            {importCopy.loadFailed}
          </p>
        ) : null}
        {sources.data === undefined ? null : (
          <SettingsCard label={importCopy.title}>
            {sources.data
              .filter((status) => status.source !== 'folder')
              .map((status) => (
                <ProviderRow key={status.source} status={status} running={running} />
              ))}
          </SettingsCard>
        )}
        <p className="text-xs text-fg-muted">{importCopy.offline}</p>
      </SettingsSection>
      <SettingsSection title={importCopy.folderTitle} description={importCopy.folderDescription}>
        <SettingsCard label={importCopy.folderTitle}>
          <FolderRow running={running} />
        </SettingsCard>
      </SettingsSection>
    </div>
  );
}
