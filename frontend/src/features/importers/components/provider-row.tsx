import { useState } from 'react';

import { Button } from '@/ui/button';
import { Input } from '@/ui/input';

import { importCopy, sourceCopy } from '../copy';
import { useForgetImportKey, useSaveImportKey } from '../queries';
import type { ImportSourceStatus } from '../types';
import { PreviewActions } from './preview-actions';

/** Props of {@link ProviderRow}. */
interface ProviderRowProps {
  status: ImportSourceStatus;
  running: boolean;
}

/** One note-taker: its key, then Check and Import once the key is saved. */
export function ProviderRow({ status, running }: ProviderRowProps) {
  const copy = sourceCopy[status.source];
  const forget = useForgetImportKey();
  const ready = status.key === 'stored';
  return (
    <div className="flex flex-col gap-3 px-4 py-3 sm:flex-row sm:items-start sm:justify-between">
      <div className="min-w-0">
        <p className="text-sm text-fg">{copy.name}</p>
        <p className="mt-0.5 text-xs text-fg-muted">
          {ready ? importCopy.keySaved : status.key === 'locked' ? importCopy.keyLocked : copy.hint}
        </p>
        {ready ? (
          <Button
            variant="link"
            size="sm"
            className="mt-1 h-auto px-0 text-xs text-fg-muted"
            disabled={forget.isPending || running}
            onClick={() => {
              forget.mutate(status.source);
            }}
          >
            {importCopy.forgetKey}
          </Button>
        ) : null}
      </div>
      {ready ? (
        <PreviewActions target={{ source: status.source, folder: null }} running={running} />
      ) : (
        <KeyField status={status} />
      )}
    </div>
  );
}

/** The key box and its Save button. */
function KeyField({ status }: { status: ImportSourceStatus }) {
  const [draft, setDraft] = useState('');
  const save = useSaveImportKey();
  const copy = sourceCopy[status.source];
  const id = `import-key-${status.source}`;
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-2">
        <Input
          id={id}
          type="password"
          autoComplete="off"
          aria-label={importCopy.keyLabel(copy.name)}
          placeholder={copy.placeholder}
          className="w-48 font-mono"
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value);
          }}
        />
        <Button
          variant="secondary"
          size="sm"
          disabled={draft.trim() === '' || save.isPending}
          onClick={() => {
            save.mutate({ source: status.source, key: draft.trim() });
          }}
        >
          {importCopy.saveKey}
        </Button>
      </div>
      {save.error === null ? null : (
        <p role="alert" className="text-xs text-danger">
          {save.error.message}
        </p>
      )}
    </div>
  );
}
