/**
 * Saving a settings change from a settings section: silent on success
 * (UX-31: the switch itself shows the change), a toast only on failure.
 */
import { toast } from 'sonner';

import { settingsCopy } from '../copy';
import { usePatchSettings } from '../queries';
import type { SettingsPatch } from '../types';

/** Saves a merge patch; says so only when it fails. */
export function useSaveSettings() {
  const patch = usePatchSettings();
  const save = (change: SettingsPatch) => {
    patch.mutate(change, {
      onError: (error) => toast.error(settingsCopy.saveFailed, { description: error.message }),
    });
  };
  return { save, saving: patch.isPending };
}
