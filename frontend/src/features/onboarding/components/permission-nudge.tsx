/**
 * One optional permission, asked in place (UX-22): a quiet line with an
 * Allow button, shown only while macOS has never been asked. Granted,
 * refused or unsupported, it renders nothing; Settings keeps the full row.
 */
import { useQuery } from '@tanstack/react-query';

import { Button } from '@/ui/button';
import { ICON_PROPS } from '@/ui/icons';

import type { ContextPermission } from '../ask-in-context';
import { contextCopy } from '../copy';
import { actionFor } from '../permission-actions';
import { PERMISSION_ICONS } from '../permission-icons';
import { permissionsQuery, useRequestPermission } from '../queries';

/** The line for `kind`, or nothing once it has an answer. */
export function PermissionNudge({ kind }: { kind: ContextPermission }) {
  // Read once (and on focus), without the setup step's polling.
  const permissions = useQuery({ ...permissionsQuery(), refetchInterval: false });
  const request = useRequestPermission();
  const status = permissions.data?.find((candidate) => candidate.kind === kind);
  if (status === undefined || actionFor(status.state) !== 'request') return null;
  const copy = contextCopy.kinds[kind];
  const KindIcon = PERMISSION_ICONS[kind];
  return (
    <div className="flex items-center gap-3 rounded-md px-3 py-2 text-sm">
      <KindIcon {...ICON_PROPS} aria-hidden className="shrink-0 text-fg-muted" />
      <p className="min-w-0 flex-1 text-fg-muted">
        <span className="text-fg">{copy.title}.</span> {copy.description}
      </p>
      <Button
        variant="secondary"
        size="sm"
        disabled={request.isPending}
        onClick={() => {
          request.mutate(kind);
        }}
      >
        {contextCopy.allow}
      </Button>
    </div>
  );
}
