/**
 * Settings → Screen capture (CAP-09): Screen Recording's state with the way
 * to System Settings, and the capture switches. Containers only: the cards
 * are presentational.
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { screenCopy as copy } from '../copy';
import { useOpenScreenRecordingSettings, useScreenStatus } from '../queries';
import type { ScreenSettings } from '../types';
import { CaptureCard } from './capture-card';
import { PermissionCard } from './permission-card';

/** The permission row once the status is read. */
function PermissionBody() {
  const status = useScreenStatus();
  const open = useOpenScreenRecordingSettings();
  if (status.data !== undefined) {
    return (
      <PermissionCard
        supported={status.data.supported}
        permitted={status.data.permitted}
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

/** The capture switches once the settings are loaded. */
function CaptureBody() {
  const settings = useSettings();
  const patch = usePatchSettings();
  if (settings.data === undefined) return <SkeletonText lines={3} />;
  const save = (next: Partial<ScreenSettings>) => {
    patch.mutate(
      { screen: next },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  return <CaptureCard screen={settings.data.screen} saving={patch.isPending} onChange={save} />;
}

/** The page. */
export function ScreenCaptureSettings() {
  return (
    <SettingsSection title={copy.section.title} description={copy.section.description}>
      <PermissionBody />
      <CaptureBody />
    </SettingsSection>
  );
}
