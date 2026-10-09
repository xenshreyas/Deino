import { useEffect } from "react"
import { Switch } from '@/ui/controls'
import { FolderOpen } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { useConfig } from "@/contexts/ConfigContext"

/**
 * The General tab of Settings. The notification switch reads and writes
 * `Settings.notifications` (FND-06) through `ConfigContext`.
 */
export function PreferenceSettings() {
  const {
    notificationSettings,
    storageLocations,
    loadPreferences,
    updateNotificationSettings
  } = useConfig();

  // Lazy load the storage folder paths on mount (only loads if not already cached)
  useEffect(() => {
    loadPreferences();
  }, [loadPreferences]);

  // "Notifications on" means both the started and the stopped notification are shown
  const notificationsEnabled = notificationSettings
    ? notificationSettings.preferences.showRecordingStarted &&
      notificationSettings.preferences.showRecordingStopped
    : null;

  const setNotificationsEnabled = async (enabled: boolean) => {
    if (!notificationSettings) return;
    try {
      await updateNotificationSettings({
        ...notificationSettings,
        preferences: {
          ...notificationSettings.preferences,
          showRecordingStarted: enabled,
          showRecordingStopped: enabled,
        },
      });
    } catch (error) {
      console.error('Failed to update notification settings:', error);
    }
  };

  const handleOpenFolder = async (folderType: 'database' | 'models' | 'recordings') => {
    try {
      switch (folderType) {
        case 'database':
          await invoke('open_database_folder');
          break;
        case 'models':
          await invoke('open_models_folder');
          break;
        case 'recordings':
          await invoke('open_recordings_folder');
          break;
      }

    } catch (error) {
      console.error(`Failed to open ${folderType} folder:`, error);
    }
  };

  if (notificationsEnabled === null) {
    return <div className="max-w-2xl mx-auto p-6">Loading Preferences...</div>
  }

  return (
    <div className="space-y-6">
      {/* Notifications Section */}
      <div className="bg-surface rounded-lg border border-border p-6 shadow-xs">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-fg mb-2">Notifications</h3>
            <p className="text-sm text-fg-muted">Enable or disable notifications of start and end of meeting</p>
          </div>
          <Switch checked={notificationsEnabled} onCheckedChange={setNotificationsEnabled} />
        </div>
      </div>

      {/* Data Storage Locations Section */}
      <div className="bg-surface rounded-lg border border-border p-6 shadow-xs">
        <h3 className="text-lg font-semibold text-fg mb-4">Data Storage Locations</h3>
        <p className="text-sm text-fg-muted mb-6">
          View and access where Deino stores your data
        </p>

        <div className="space-y-4">
          {/* Database Location */}
          {/* <div className="p-4 border rounded-lg bg-bg-subtle">
            <div className="font-medium mb-2">Database</div>
            <div className="text-sm text-fg-muted mb-3 break-all font-mono text-xs">
              {storageLocations?.database || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('database')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-border-strong rounded-md hover:bg-bg-muted transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Models Location */}
          {/* <div className="p-4 border rounded-lg bg-bg-subtle">
            <div className="font-medium mb-2">Whisper Models</div>
            <div className="text-sm text-fg-muted mb-3 break-all font-mono text-xs">
              {storageLocations?.models || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('models')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-border-strong rounded-md hover:bg-bg-muted transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Recordings Location */}
          <div className="p-4 border rounded-lg bg-bg-subtle">
            <div className="font-medium mb-2">Meeting Recordings</div>
            <div className="text-sm text-fg-muted mb-3 break-all font-mono text-xs">
              {storageLocations?.recordings || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('recordings')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-border-strong rounded-md hover:bg-bg-muted transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div>
        </div>

        <div className="mt-4 p-3 bg-bg-muted rounded-md">
          <p className="text-xs text-fg-muted">
            <strong>Note:</strong> Database and models are stored together in your application data directory for unified management.
          </p>
        </div>
      </div>
    </div>
  )
}
