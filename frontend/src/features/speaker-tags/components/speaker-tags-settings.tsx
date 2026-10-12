/**
 * Settings → Recording → Speaker names from call apps (CAP-05, UX-09,
 * UX-10): the apps it covers as plain text, an app marked only when it is
 * unsupported here. Accessibility itself is one row of the Permissions card
 * above; while it is missing, this section shows one "Needs Accessibility"
 * link there. The status is polled, so it updates when the user comes back
 * from System Settings.
 */
import { PermissionLink } from '@/features/settings';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { speakerTagsCopy as copy } from '../copy';
import { useSpeakerTagStatus } from '../queries';
import type { AppTagStatus, TagState } from '../types';

/**
 * The permission's state across the apps: missing when any app needs it,
 * allowed when any app can read names, otherwise not available here.
 */
export function permissionState(statuses: readonly AppTagStatus[]): TagState {
  if (statuses.some((status) => status.state === 'needsPermission')) return 'needsPermission';
  if (statuses.some((status) => status.state === 'enabled')) return 'enabled';
  return 'unsupported';
}

/** "Works with Zoom, Microsoft Teams (not supported here) and Google Meet." */
export function worksWith(statuses: readonly AppTagStatus[]): string {
  const names = statuses.map((status) => {
    const name = copy.apps[status.app].name;
    return status.state === 'unsupported' ? copy.unsupportedApp(name) : name;
  });
  const last = names.pop();
  if (last === undefined) return '';
  return copy.worksWith(names.length === 0 ? last : `${names.join(', ')} and ${last}`);
}

/** Props of {@link SpeakerTagsStatus}. */
export interface SpeakerTagsStatusProps {
  statuses: readonly AppTagStatus[];
}

/** The apps, and the permission link while Accessibility is missing. Presentational. */
export function SpeakerTagsStatus({ statuses }: SpeakerTagsStatusProps) {
  const state = permissionState(statuses);
  return (
    <>
      <p className="text-sm text-fg-muted">
        {state === 'unsupported' ? copy.unavailable : worksWith(statuses)}
      </p>
      {state === 'needsPermission' ? <PermissionLink permission={copy.permission} /> : null}
    </>
  );
}

/** The status once loaded, a skeleton before, an inline error on failure. */
function StatusBody() {
  const status = useSpeakerTagStatus();
  if (status.data !== undefined) return <SpeakerTagsStatus statuses={status.data} />;
  if (status.isError) {
    return (
      <p role="alert" className="text-xs text-danger">
        {copy.loadFailed}
      </p>
    );
  }
  return <SkeletonText lines={1} />;
}

/** The section. */
export function SpeakerTagsSettings() {
  return (
    <SettingsSection title={copy.section.title} description={copy.section.description}>
      <StatusBody />
    </SettingsSection>
  );
}
