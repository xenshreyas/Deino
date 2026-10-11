/** Labels the Integrations page builds from Rust's values. */
import { differenceInMinutes, formatDistanceStrict } from 'date-fns';

import { integrationsCopy } from './copy';
import type { Capability, ConnectMethod } from './types';

/** "Posts messages · Creates pages". */
export function capabilitySummary(capabilities: readonly Capability[]): string {
  return capabilities.map((capability) => integrationsCopy.capabilities[capability]).join(' · ');
}

/** "just now", or "5 minutes ago". */
export function relativeTime(at: string, now: Date): string {
  const then = new Date(at);
  if (differenceInMinutes(now, then) < 1) return integrationsCopy.justNow;
  return formatDistanceStrict(then, now, { addSuffix: true });
}

/** "Last used 5 minutes ago", or "Not used yet". */
export function lastUsedLabel(lastUsedAt: string | null, now: Date): string {
  if (lastUsedAt === null) return integrationsCopy.connection.neverUsed;
  return integrationsCopy.connection.lastUsed(relativeTime(lastUsedAt, now));
}

/** The token paste method, if the provider offers one. */
export function tokenMethod(
  methods: readonly ConnectMethod[],
): Extract<ConnectMethod, { method: 'token' }> | undefined {
  return methods.find(
    (method): method is Extract<ConnectMethod, { method: 'token' }> => method.method === 'token',
  );
}

/** Whether the provider signs in through the browser. */
export function hasOAuth(methods: readonly ConnectMethod[]): boolean {
  return methods.some((method) => method.method === 'oauth');
}
