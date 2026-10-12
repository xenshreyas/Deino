/**
 * Settings → Speaker tags (CAP-05): a row per meeting app with its state,
 * and the way to System Settings when Accessibility is missing. Deino never
 * asks macOS for the permission itself; the page polls the status, so it
 * turns Enabled when the user comes back from System Settings.
 */
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { speakerTagsCopy as copy } from '../copy';
import { useOpenAccessibilitySettings, useSpeakerTagStatus } from '../queries';
import type { AppTagStatus } from '../types';
import { AppTagRow } from './app-tag-row';

/** Props of {@link SpeakerTagsCard}. */
export interface SpeakerTagsCardProps {
  statuses: readonly AppTagStatus[];
  opening: boolean;
  /** Opening System Settings failed. */
  openFailed: boolean;
  onOpenSettings: () => void;
}

/** The rows, and what to do when permission is missing. Presentational. */
export function SpeakerTagsCard({
  statuses,
  opening,
  openFailed,
  onOpenSettings,
}: SpeakerTagsCardProps) {
  const needsPermission = statuses.some((status) => status.state === 'needsPermission');
  return (
    <>
      <SettingsCard label={copy.section.title}>
        {statuses.map((status) => (
          <AppTagRow
            key={status.app}
            status={status}
            opening={opening}
            onOpenSettings={onOpenSettings}
          />
        ))}
      </SettingsCard>
      {needsPermission ? <p className="px-1 text-xs text-fg-muted">{copy.permissionHint}</p> : null}
      {openFailed ? (
        <p role="alert" className="px-1 text-xs text-danger">
          {copy.settingsFailed}
        </p>
      ) : null}
    </>
  );
}

/** The rows once loaded, a skeleton before, an inline error on failure. */
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
  return <SkeletonText lines={3} />;
}

/** The page: the meeting apps and their speaker-tag states. */
export function SpeakerTagsSettings() {
  return (
    <SettingsSection title={copy.section.title} description={copy.section.description}>
      <StatusBody />
    </SettingsSection>
  );
}
