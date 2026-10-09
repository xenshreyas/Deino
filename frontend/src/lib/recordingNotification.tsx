import { toast } from 'sonner';

import { settingsApi } from '@/features/settings';

/**
 * Shows the recording notification toast with compliance message.
 * Checks the "tell participants" setting (`recording.showRecordingNotice`)
 * and displays a dismissible toast with:
 * - notice to inform participants
 * - "Don't show again" checkbox, which turns the setting off
 * - Acknowledgment button
 *
 * @returns Promise<void> - Resolves when notification is shown or skipped
 */
export async function showRecordingNotification(): Promise<void> {
  try {
    const { recording } = await settingsApi.get();

    if (recording.showRecordingNotice) {
      let dontShowAgain = false;

      const toastId = toast.info('🔴 Recording Started', {
        description: (
          <div className="space-y-3 min-w-[280px]">
            <p className="text-sm font-medium text-gray-900">
              Inform all participants this meeting is being recorded.
            </p>
            <label className="flex items-center gap-2 text-xs cursor-pointer hover:bg-blue-100 p-2 rounded transition-colors">
              <input
                type="checkbox"
                onChange={(e) => {
                  dontShowAgain = e.target.checked;
                }}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 focus:ring-2"
              />
              <span className="select-none text-gray-700">Don't show this again</span>
            </label>
            <button
              onClick={async () => {
                if (dontShowAgain) {
                  await settingsApi.patch({ recording: { showRecordingNotice: false } });
                }
                toast.dismiss(toastId);
              }}
              className="w-full px-3 py-1.5 bg-gray-900 text-white text-xs rounded hover:bg-gray-800 transition-colors font-medium"
            >
              I've Notified Participants
            </button>
          </div>
        ),
        duration: 10000,
        position: 'bottom-right',
      });
    }
  } catch (notificationError) {
    console.error('Failed to show recording notification:', notificationError);
    // Don't fail the recording if notification fails
  }
}
