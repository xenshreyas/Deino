/**
 * Settings → Recording → Screen capture (CAP-09): one switch, the rest under
 * "More options" (UX-15). Screen Recording itself is a row of the
 * Permissions card (UX-10); this section only links there while capture is
 * on and the permission is missing. Container: the card is presentational.
 */
import { toast } from 'sonner';

import { PermissionLink, usePatchSettings, useSettings } from '@/features/settings';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { screenCopy as copy } from '../copy';
import { useScreenStatus } from '../queries';
import type { ScreenSettings } from '../types';
import { CaptureCard } from './capture-card';

/** "Needs Screen Recording" while capture is on and macOS has not allowed it. */
function MissingPermission({ on }: { on: boolean }) {
  const status = useScreenStatus();
  const missing = status.data?.supported === true && !status.data.permitted;
  return on && missing ? <PermissionLink permission={copy.permission} /> : null;
}

/** The capture switches once the settings are loaded. */
function CaptureBody() {
  const settings = useSettings();
  const patch = usePatchSettings();
  if (settings.data === undefined) return <SkeletonText lines={2} />;
  const save = (next: Partial<ScreenSettings>) => {
    patch.mutate(
      { screen: next },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  const { screen } = settings.data;
  return (
    <>
      <CaptureCard screen={screen} saving={patch.isPending} onChange={save} />
      <MissingPermission on={screen.detectedMeetings} />
    </>
  );
}

/** The section. */
export function ScreenCaptureSettings() {
  return (
    <SettingsSection title={copy.section.title} description={copy.section.description}>
      <CaptureBody />
    </SettingsSection>
  );
}
