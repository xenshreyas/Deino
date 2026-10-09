import { useSettings } from '@/features/settings';
import { SkeletonText } from '@/ui/skeleton';

import { AccountExport } from './account-export';
import { AutoExportSettings } from './auto-export-settings';

/**
 * Settings → Export (EXP-02): auto-export of every meeting into a folder,
 * and the export of every meeting at once.
 */
export function ExportSettingsPage() {
  const settings = useSettings();
  if (settings.data === undefined) return <SkeletonText lines={8} />;
  return (
    <div className="flex flex-col gap-8">
      <AutoExportSettings settings={settings.data.export} />
      <AccountExport />
    </div>
  );
}
