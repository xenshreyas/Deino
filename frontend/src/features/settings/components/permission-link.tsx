/**
 * The one line a feature shows while a macOS permission it needs is missing
 * (UX-10): "Needs Accessibility", linking to the Permissions card at the top
 * of Settings → Recording, where the permission's only row lives.
 */
import { Link } from '@tanstack/react-router';

import { ICON_PROPS, Info } from '@/ui/icons';

import { permissionLinkCopy as copy } from '../copy';

/** Props of {@link PermissionLink}. */
export interface PermissionLinkProps {
  /** The permission as macOS names it: "Accessibility", "Screen Recording". */
  permission: string;
}

/** "Needs Screen Recording", a link to the Permissions card. */
export function PermissionLink({ permission }: PermissionLinkProps) {
  return (
    <p className="flex items-center gap-1.5 px-1 text-xs text-fg-muted">
      <Info {...ICON_PROPS} aria-hidden className="shrink-0 text-warning-text" />
      <Link
        to="/settings"
        search={{ section: 'permissions' }}
        className="underline-offset-4 hover:text-fg hover:underline"
      >
        {copy.needs(permission)}
      </Link>
    </p>
  );
}
