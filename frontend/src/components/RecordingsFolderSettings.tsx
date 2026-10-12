import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

import { useConfig } from '@/contexts/ConfigContext';
import { Button } from '@/ui/button';
import { FolderOpen, ICON_PROPS } from '@/ui/icons';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

/**
 * Settings → General → Recordings folder (UX-20): where the audio of each
 * meeting is kept, and a way to show it in Finder. The folder path and the
 * Finder call are still legacy commands, so this panel stays legacy until a
 * card ports them; it is drawn with the settings primitives. Notifications
 * moved to their own section (UX-17).
 */
export function RecordingsFolderSettings() {
  const { storageLocations, loadPreferences } = useConfig();

  // Loads the folder paths once (cached by ConfigContext).
  useEffect(() => {
    void loadPreferences();
  }, [loadPreferences]);

  const openFolder = async () => {
    try {
      await invoke('open_recordings_folder');
    } catch (error) {
      console.error('Failed to open the recordings folder:', error);
    }
  };

  return (
    <SettingsSection
      title="Recordings folder"
      description="Where Deino keeps the audio of each meeting. Privacy decides when it is deleted."
    >
      {storageLocations === null ? (
        <SkeletonText lines={1} />
      ) : (
        <SettingsCard label="Recordings folder">
          <SettingsRow label="Folder" description={storageLocations.recordings}>
            <Button variant="secondary" size="sm" onClick={() => void openFolder()}>
              <FolderOpen {...ICON_PROPS} />
              Show in Finder
            </Button>
          </SettingsRow>
        </SettingsCard>
      )}
    </SettingsSection>
  );
}
