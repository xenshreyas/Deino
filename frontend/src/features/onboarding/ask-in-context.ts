/**
 * Optional permissions asked where they matter (UX-22), not during setup.
 * {@link askPermissionOnce} is for moments that happen away from Deino's
 * window (a Zoom call starting): one toast with an Allow button, at most
 * once per launch, and only while macOS has never been asked.
 */
import { toast } from 'sonner';

import { onboardingApi } from './api';
import { contextCopy } from './copy';
import { actionFor } from './permission-actions';
import type { PermissionKind } from './types';

/** A permission asked this way. */
export type ContextPermission = keyof typeof contextCopy.kinds;

/** How long the ask stays up: long enough to notice after joining a call. */
const ASK_DURATION_MS = 15_000;

/** The permissions already offered since launch. */
const offered = new Set<PermissionKind>();

/** Whether `kind` still needs its first answer from the user. */
async function needsAsking(kind: PermissionKind): Promise<boolean> {
  const statuses = await onboardingApi.permissions();
  const status = statuses.find((candidate) => candidate.kind === kind);
  return status !== undefined && actionFor(status.state) === 'request';
}

/**
 * Offers `kind` once per launch, if macOS has not been asked yet. A refused
 * permission is left alone: Settings → Recording has its row.
 */
export async function askPermissionOnce(kind: ContextPermission): Promise<void> {
  if (offered.has(kind)) return;
  offered.add(kind);
  if (!(await needsAsking(kind))) return;
  const copy = contextCopy.kinds[kind];
  toast(copy.title, {
    description: copy.description,
    duration: ASK_DURATION_MS,
    action: {
      label: contextCopy.allow,
      onClick: () => void onboardingApi.requestPermission(kind),
    },
  });
}
