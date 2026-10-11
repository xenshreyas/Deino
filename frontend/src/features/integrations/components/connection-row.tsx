/**
 * One connected account: who it is, whether it works, when it was last
 * used, what it may access, and its last error. Presentational.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';

import { integrationsCopy } from '../copy';
import { lastUsedLabel } from '../format';
import type { Connection } from '../types';

const copy = integrationsCopy.connection;

/** Props of {@link ConnectionRow}. */
export interface ConnectionRowProps {
  connection: Connection;
  now: Date;
  /** A check or disconnect of this connection is in flight. */
  busy: boolean;
  onCheck: () => void;
  onDisconnect: () => void;
}

/** The status dot and word: calm when connected, warning when not. */
function Status({ connection }: { connection: Connection }) {
  const healthy = connection.status === 'connected';
  return (
    <span className="inline-flex items-center gap-1.5">
      <span
        aria-hidden
        className={cn('size-1.5 rounded-full', healthy ? 'bg-success' : 'bg-warning')}
      />
      <span className={healthy ? 'text-fg-muted' : 'text-warning-text'}>
        {integrationsCopy.statuses[connection.status]}
      </span>
    </span>
  );
}

/** The details line: status, last use and granted scopes. */
function Details({ connection, now }: Pick<ConnectionRowProps, 'connection' | 'now'>) {
  return (
    <p className="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-fg-muted">
      <Status connection={connection} />
      <span aria-hidden>·</span>
      <span>{lastUsedLabel(connection.lastUsedAt, now)}</span>
      {connection.scopes.length === 0 ? null : (
        <>
          <span aria-hidden>·</span>
          <span className="truncate font-mono">{copy.access(connection.scopes.join(', '))}</span>
        </>
      )}
    </p>
  );
}

/** One connected account. */
export function ConnectionRow({
  connection,
  now,
  busy,
  onCheck,
  onDisconnect,
}: ConnectionRowProps) {
  return (
    <div className="px-4 py-3">
      <div className="grid grid-cols-[1fr_auto] items-center gap-4">
        <div className="min-w-0">
          <p className="truncate text-sm text-fg">{connection.accountLabel}</p>
          <Details connection={connection} now={now} />
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="sm" disabled={busy} onClick={onCheck}>
            {copy.check}
          </Button>
          <Button variant="secondary" size="sm" disabled={busy} onClick={onDisconnect}>
            {copy.disconnect}
          </Button>
        </div>
      </div>
      {connection.lastError === null ? null : (
        <p role="alert" className="mt-2 text-xs text-danger">
          {connection.lastError.message}
        </p>
      )}
    </div>
  );
}
