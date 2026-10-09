import { useId, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Progress, Switch } from '@/ui/controls';
import { Archive, CircleCheck, ICON_PROPS } from '@/ui/icons';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';

import { exportCopy } from '../copy';
import { messageOf } from '../hooks/use-share-actions';
import { useCancelFullExport, useFullExportStatus, useStartFullExport } from '../queries';
import type { FullExportStatus } from '../types';

/** A finished, cancelled or failed export's one-line outcome. */
function outcomeOf(status: FullExportStatus | undefined): string | null {
  switch (status?.state) {
    case 'finished':
      return exportCopy.accountFinished(status.fileName ?? '');
    case 'cancelled':
      return exportCopy.accountCancelled;
    case 'failed':
      return status.message ?? exportCopy.accountFailed;
    default:
      return null;
  }
}

/** The running export: its count, Cancel and a progress bar. */
function RunningRow({ status }: { status: FullExportStatus }) {
  const cancel = useCancelFullExport();
  const percent = status.total === 0 ? 0 : Math.round((status.done / status.total) * 100);
  return (
    <div className="flex flex-col gap-2 px-4 py-3" aria-live="polite">
      <div className="flex items-center justify-between gap-4">
        <span className="text-sm text-fg tabular-nums">
          {exportCopy.accountProgress(status.done, status.total)}
        </span>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            cancel.mutate();
          }}
        >
          {exportCopy.accountCancel}
        </Button>
      </div>
      <Progress value={percent} aria-label={exportCopy.accountTitle} />
    </div>
  );
}

/** The last outcome, then the button that starts an export. */
function StartRow({
  status,
  includeAudio,
}: {
  status: FullExportStatus | undefined;
  includeAudio: boolean;
}) {
  const start = useStartFullExport();
  const begin = () => {
    start.mutate(includeAudio, { onError: (error) => toast.error(messageOf(error)) });
  };
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <span className="flex items-center gap-1.5 text-xs text-fg-muted" aria-live="polite">
        {status?.state === 'finished' ? (
          <CircleCheck {...ICON_PROPS} className="text-success" />
        ) : null}
        {outcomeOf(status)}
      </span>
      <Button
        variant="secondary"
        size="sm"
        disabled={status?.state === 'running' || start.isPending}
        onClick={begin}
      >
        <Archive {...ICON_PROPS} />
        {exportCopy.accountStart}
      </Button>
    </div>
  );
}

/** Settings → Export → Export everything (EXP-02). */
export function AccountExport() {
  const id = useId();
  const [includeAudio, setIncludeAudio] = useState(false);
  const status = useFullExportStatus().data;
  const running = status?.state === 'running';
  return (
    <SettingsSection title={exportCopy.accountTitle} description={exportCopy.accountDescription}>
      <SettingsCard label={exportCopy.accountTitle}>
        <SettingsRow
          label={exportCopy.accountAudio}
          description={exportCopy.accountAudioHint}
          htmlFor={id}
        >
          <Switch
            id={id}
            checked={includeAudio}
            onCheckedChange={setIncludeAudio}
            disabled={running}
          />
        </SettingsRow>
        {running ? <RunningRow status={status} /> : null}
        <StartRow status={status} includeAudio={includeAudio} />
      </SettingsCard>
    </SettingsSection>
  );
}
