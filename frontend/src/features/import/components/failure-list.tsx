/** Files watched folders could not import: name, reason, Retry and Dismiss. */
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { SettingsCard } from '@/ui/settings-section';

import { importCopy as copy } from '../copy';
import { useResolveFailure } from '../queries';
import type { WatchFailure } from '../types';

/** The file's name from its path. */
function fileName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

/** One failed file. */
function FailureRow({ failure }: { failure: WatchFailure }) {
  const resolve = useResolveFailure();
  const run = (action: 'retry' | 'dismiss') => {
    resolve.mutate(
      { id: failure.id, action },
      { onError: (error) => toast.error(copy.changeFailed, { description: error.message }) },
    );
  };
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <CircleAlert {...ICON_PROPS} className="shrink-0 text-danger" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm text-fg" title={failure.path}>
          {fileName(failure.path)}
        </p>
        <p className="text-xs text-fg-muted">{copy.failure[failure.failure]}</p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        disabled={resolve.isPending}
        onClick={() => {
          run('dismiss');
        }}
      >
        {copy.dismiss}
      </Button>
      <Button
        variant="secondary"
        size="sm"
        disabled={resolve.isPending}
        onClick={() => {
          run('retry');
        }}
      >
        {copy.retry}
      </Button>
    </div>
  );
}

/** Every failure, or a calm line when there are none. */
export function FailureList({ failures }: { failures: readonly WatchFailure[] }) {
  if (failures.length === 0) return <p className="text-xs text-fg-muted">{copy.noFailures}</p>;
  return (
    <SettingsCard label={copy.failuresTitle}>
      {failures.map((failure) => (
        <FailureRow key={failure.id} failure={failure} />
      ))}
    </SettingsCard>
  );
}
