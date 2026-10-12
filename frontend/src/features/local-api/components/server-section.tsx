import type { UseMutationResult } from '@tanstack/react-query';
import { useState } from 'react';

import type { IpcError } from '@/lib/ipc/result';
import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { Input } from '@/ui/input';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { Skeleton } from '@/ui/skeleton';

import { apiCopy } from '../copy';
import { useApiStatus, useConfigureApi } from '../queries';
import type { ApiSettings, ApiStatus } from '../types';

/** The lowest and highest port the server may use. */
const MIN_PORT = 1024;
const MAX_PORT = 65_535;

/** `text` as a usable port, or `null`. */
export function parsePort(text: string): number | null {
  const port = Number(text.trim());
  return Number.isInteger(port) && port >= MIN_PORT && port <= MAX_PORT ? port : null;
}

/** The local API's switch, port and address (container). */
export function ServerSection() {
  const status = useApiStatus();
  const configure = useConfigureApi();
  if (status.isPending) return <Skeleton className="h-28 w-full" />;
  if (status.isError) {
    return (
      <p role="alert" className="text-sm text-danger">
        {apiCopy.loadFailed}
      </p>
    );
  }
  // Remount on a saved port, so the field starts from the saved value.
  return <ServerForm key={status.data.port} status={status.data} configure={configure} />;
}

/** Props of {@link ServerForm}. */
interface ServerFormProps {
  status: ApiStatus;
  configure: UseMutationResult<ApiStatus, IpcError, ApiSettings>;
}

function ServerForm({ status, configure }: ServerFormProps) {
  const [draft, setDraft] = useState(String(status.port));
  const port = parsePort(draft);
  const busy = configure.isPending;
  return (
    <SettingsSection title={apiCopy.title} description={apiCopy.description}>
      <SettingsCard label={apiCopy.title}>
        <SettingsRow label={apiCopy.enable} description={apiCopy.enableHint} htmlFor="api-enabled">
          <Switch
            id="api-enabled"
            checked={status.enabled}
            disabled={busy}
            onCheckedChange={(enabled) => {
              configure.mutate({ enabled, port: status.port });
            }}
          />
        </SettingsRow>
        <SettingsRow label={apiCopy.port} description={apiCopy.portHint} htmlFor="api-port">
          <PortField
            draft={draft}
            onDraft={setDraft}
            canSave={!busy && port !== null && port !== status.port}
            onSave={() => {
              if (port !== null) configure.mutate({ enabled: status.enabled, port });
            }}
          />
        </SettingsRow>
      </SettingsCard>
      <ServerState status={status} error={configure.error} invalid={port === null} />
    </SettingsSection>
  );
}

/** Props of {@link PortField}. */
interface PortFieldProps {
  draft: string;
  onDraft: (draft: string) => void;
  canSave: boolean;
  onSave: () => void;
}

function PortField({ draft, onDraft, canSave, onSave }: PortFieldProps) {
  return (
    <div className="flex items-center gap-2">
      <Input
        id="api-port"
        inputMode="numeric"
        className="w-24 font-mono"
        value={draft}
        onChange={(event) => {
          onDraft(event.target.value);
        }}
      />
      <Button variant="secondary" size="sm" disabled={!canSave} onClick={onSave}>
        {apiCopy.save}
      </Button>
    </div>
  );
}

/** Props of {@link ServerState}. */
interface ServerStateProps {
  status: ApiStatus;
  error: IpcError | null;
  invalid: boolean;
}

/** Where it listens and how to call it, or why it does not. */
function ServerState({ status, error, invalid }: ServerStateProps) {
  const problem = invalid ? apiCopy.portInvalid : (error?.message ?? status.error);
  return (
    <div className="flex flex-col gap-1 text-xs text-fg-muted">
      {problem === null ? null : (
        <p role="alert" className="text-sm break-words text-danger">
          {problem}
        </p>
      )}
      {status.baseUrl === null ? null : (
        <>
          <p role="status" className="font-mono text-fg">
            {apiCopy.listening(status.baseUrl)}
          </p>
          <p>{apiCopy.docs}</p>
          <p>{apiCopy.locked}</p>
        </>
      )}
    </div>
  );
}
