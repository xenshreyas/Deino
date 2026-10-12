import { cn } from '@/ui/cn';
import { Button } from '@/ui/button';

import { apiCopy } from '../copy';
import type { ApiKey } from '../types';

/** An ISO time as "Oct 12, 2026". */
export function day(iso: string): string {
  return new Date(iso).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

/** What the row says under the key's name. */
export function keyDetails(key: ApiKey): string {
  const used = key.lastUsedAt === null ? apiCopy.neverUsed : apiCopy.lastUsed(day(key.lastUsedAt));
  return `${apiCopy.scopes[key.scope]} · ${apiCopy.created(day(key.createdAt))}, ${used}`;
}

/** Props of {@link KeyRow}. */
export interface KeyRowProps {
  apiKey: ApiKey;
  busy: boolean;
  onRevoke: () => void;
  onRemove: () => void;
}

/** One key: its name and use, and Revoke (or Remove once revoked). */
export function KeyRow({ apiKey, busy, onRevoke, onRemove }: KeyRowProps) {
  const revoked = apiKey.revokedAt !== null;
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className={cn('min-w-0', revoked && 'opacity-60')}>
        <p className="truncate text-sm text-fg">
          {apiKey.name}
          {revoked ? <span className="ml-2 text-xs text-fg-muted">{apiCopy.revoked}</span> : null}
        </p>
        <p className="mt-0.5 text-xs text-fg-muted">{keyDetails(apiKey)}</p>
      </div>
      {revoked ? (
        <Button variant="ghost" size="sm" disabled={busy} onClick={onRemove}>
          {apiCopy.remove}
        </Button>
      ) : (
        <Button variant="danger" size="sm" disabled={busy} onClick={onRevoke}>
          {apiCopy.revoke}
        </Button>
      )}
    </div>
  );
}
