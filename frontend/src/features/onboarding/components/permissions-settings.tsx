/**
 * Settings → Recording → Permissions (UX-10): every macOS permission Deino
 * uses, once, with onboarding's rows. Features that need one show a
 * one-line "Needs …" link here instead of their own row. The list re-reads
 * itself, so a switch flipped in System Settings shows up.
 */
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { permissionsSettingsCopy as copy } from '../copy';
import { usePermissions } from '../queries';
import { CalendarPermission, SystemPermission } from './permission-rows';
import { SetupList } from './setup-row';

/** The Permissions card. */
export function PermissionsSettings() {
  const permissions = usePermissions();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {permissions.data === undefined ? (
        <SkeletonText lines={5} />
      ) : (
        <div className="rounded-lg border border-border bg-surface px-4 [&>ul]:border-y-0">
          <SetupList label={copy.title}>
            {permissions.data.map((status) => (
              <SystemPermission key={status.kind} status={status} showOptional={false} />
            ))}
            <CalendarPermission showOptional={false} />
          </SetupList>
        </div>
      )}
      {permissions.isError ? (
        <p role="alert" className="text-xs text-danger">
          {copy.loadFailed}
        </p>
      ) : null}
    </SettingsSection>
  );
}
