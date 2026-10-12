/**
 * Settings → Recording → Speaker names from call apps (CAP-05, UX-09): one
 * Accessibility row for Zoom, Teams and Meet, and the apps as plain text.
 * Deino never asks macOS for the permission itself; the section polls the
 * status, so it turns Allowed when the user comes back from System Settings.
 */
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { speakerTagsCopy as copy } from '../copy';
import { useOpenAccessibilitySettings, useSpeakerTagStatus } from '../queries';
import type { AppTagStatus } from '../types';
import { AccessibilityRow, permissionState, worksWith } from './accessibility-row';

/** Props of {@link SpeakerTagsCard}. */
export interface SpeakerTagsCardProps {
  statuses: readonly AppTagStatus[];
  opening: boolean;
  /** Opening System Settings failed. */
  openFailed: boolean;
  onOpenSettings: () => void;
}

/** The row, the apps, and what to do when permission is missing. Presentational. */
export function SpeakerTagsCard({ statuses, opening, openFailed, onOpenSettings }: SpeakerTagsCardProps) {
  const state = permissionState(statuses);
  return (
    <>
      <SettingsCard label={copy.section.title}>
        <AccessibilityRow state={state} opening={opening} onOpenSettings={onOpenSettings} />
      </SettingsCard>
      <p className="px-1 text-xs text-fg-muted">
        {worksWith(statuses)}
        {state === 'needsPermission' ? ` ${copy.permissionHint}` : null}
      </p>
      {openFailed ? (
        <p role="alert" className="px-1 text-xs text-danger">
          {copy.settingsFailed}
        </p>
      ) : null}
    </>
  );
}

/** The row once loaded, a skeleton before, an inline error on failure. */
function StatusBody() {
  const status = useSpeakerTagStatus();
  const open = useOpenAccessibilitySettings();
  if (status.data !== undefined) {
    return (
      <SpeakerTagsCard
        statuses={status.data}
        opening={open.isPending}
        openFailed={open.isError}
        onOpenSettings={() => {
          open.mutate();
        }}
      />
    );
  }
  if (status.isError) {
    return (
      <p role="alert" className="text-xs text-danger">
        {copy.loadFailed}
      </p>
    );
  }
  return <SkeletonText lines={2} />;
}

/** The section: Accessibility for speaker names, and the apps it covers. */
export function SpeakerTagsSettings() {
  return (
    <SettingsSection title={copy.section.title} description={copy.section.description}>
      <StatusBody />
    </SettingsSection>
  );
}
