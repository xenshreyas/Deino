import { Switch } from '@/components/ui/switch';
import { FolderOpen } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { DeviceSelection, SelectedDevices } from '@/components/DeviceSelection';
import { toast } from 'sonner';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { type SettingsPatch, usePatchSettings, useSettings } from '@/features/settings';

/**
 * The Recordings tab of Settings: keep audio, the "tell participants"
 * notice and the default devices, all stored in the typed settings
 * (FND-06). Each switch saves at once.
 */
export function RecordingSettings() {
  const { data: settings } = useSettings();
  const { mutateAsync: patch, isPending: saving } = usePatchSettings();
  const { isRecording } = useRecordingState();
  const { data: recordingsFolder } = useQuery({
    queryKey: ['recordings-folder'],
    queryFn: () => invoke<string>('get_default_recordings_folder_path'),
  });

  const save = async (change: SettingsPatch, success: string, description?: string) => {
    try {
      await patch(change);
      toast.success(success, description === undefined ? undefined : { description });
    } catch (error) {
      console.error('Failed to save recording settings:', error);
      toast.error('Failed to save preference', {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const handleDeviceChange = (devices: SelectedDevices) =>
    save(
      { audio: { preferredMicDevice: devices.micDevice, preferredSystemDevice: devices.systemDevice } },
      'Device preferences saved',
      `Microphone: ${devices.micDevice || 'Default'}, System Audio: ${devices.systemDevice || 'Default'}`,
    );

  const handleOpenFolder = async () => {
    try {
      await invoke('open_recordings_folder');
    } catch (error) {
      console.error('Failed to open recordings folder:', error);
    }
  };

  if (!settings) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-gray-200 rounded w-1/4 mb-4"></div>
        <div className="h-8 bg-gray-200 rounded mb-4"></div>
      </div>
    );
  }

  const { autoSave, showRecordingNotice } = settings.recording;

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">Recording Settings</h3>
        <p className="text-sm text-gray-600 mb-6">
          Configure how your audio recordings are saved during meetings.
        </p>
      </div>

      {/* Auto Save Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Save Audio Recordings</div>
          <div className="text-sm text-gray-600">
            Automatically save audio files when recording stops
          </div>
        </div>
        <Switch
          checked={autoSave}
          onCheckedChange={(enabled) => save({ recording: { autoSave: enabled } }, 'Preference saved')}
          disabled={saving}
        />
      </div>

      {/* Folder Location - Only shown when auto_save is enabled */}
      {autoSave && (
        <div className="space-y-4">
          <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Save Location</div>
            <div className="text-sm text-gray-600 mb-3 break-all">
              {recordingsFolder || 'Default folder'}
            </div>
            <button
              onClick={handleOpenFolder}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-50 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div>

          <div className="p-4 border rounded-lg bg-blue-50">
            <div className="text-sm text-blue-800">
              <strong>File Format:</strong> MP4 files
            </div>
          </div>
        </div>
      )}

      {/* Info when auto_save is disabled */}
      {!autoSave && (
        <div className="p-4 border rounded-lg bg-yellow-50">
          <div className="text-sm text-yellow-800">
            Audio recording is disabled. Enable "Save Audio Recordings" to automatically save your meeting audio.
          </div>
        </div>
      )}

      {/* Recording Notification Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Recording Start Notification</div>
          <div className="text-sm text-gray-600">
            Show reminder to inform participants when recording starts
          </div>
        </div>
        <Switch
          checked={showRecordingNotice}
          onCheckedChange={(enabled) => save({ recording: { showRecordingNotice: enabled } }, 'Preference saved')}
        />
      </div>

      {/* Device Preferences */}
      <div className="space-y-4">
        <div className="border-t pt-6">
          <h4 className="text-base font-medium text-gray-900 mb-4">Default Audio Devices</h4>
          <p className="text-sm text-gray-600 mb-4">
            Set your preferred microphone and system audio devices for recording. These will be automatically selected when starting new recordings.
          </p>

          {isRecording && (
            <p
              role="status"
              aria-live="polite"
              className="text-sm text-amber-700 bg-amber-50 border border-amber-200 rounded-md p-2 mb-4"
            >
              Device selection is locked while a recording is in progress. Connecting a new device mid-recording will not switch to it. Stop the current meeting to change devices.
            </p>
          )}

          <div className="border rounded-lg p-4 bg-gray-50">
            <DeviceSelection
              selectedDevices={{
                micDevice: settings.audio.preferredMicDevice,
                systemDevice: settings.audio.preferredSystemDevice,
              }}
              onDeviceChange={handleDeviceChange}
              disabled={saving || isRecording}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
