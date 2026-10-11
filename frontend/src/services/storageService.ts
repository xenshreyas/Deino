/**
 * Storage Service
 *
 * Deletes meetings through the legacy `api_delete_meeting` command. Reading
 * meetings goes through `@/features/meetings` (the `meetings` plugin); Rust
 * saves recordings itself (FND-05b).
 */

import { invoke } from '@tauri-apps/api/core';

export class StorageService {
  /**
   * Delete a meeting and everything stored with it (legacy `api_delete_meeting`,
   * until ORG-07 ports it to the `meetings` plugin with a trash).
   */
  async deleteMeeting(meetingId: string): Promise<void> {
    await invoke('api_delete_meeting', { meetingId });
  }
}

// Export singleton instance
export const storageService = new StorageService();
