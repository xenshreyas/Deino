/**
 * Settings → General → Theme: light, dark or the system's, for every Deino
 * window.
 */
import { appearanceCopy, ThemePicker } from '@/features/appearance';
import { SettingsSection } from '@/ui/settings-section';

import { useSaveTheme, useThemePreference } from '../theme';

/** The theme picker. */
export function AppearanceSection() {
  const preference = useThemePreference();
  const saveTheme = useSaveTheme();
  return (
    <SettingsSection
      title={appearanceCopy.themeLabel}
      description={appearanceCopy.themeDescription}
    >
      <ThemePicker value={preference} onChange={saveTheme} />
    </SettingsSection>
  );
}
