/**
 * Update Service
 *
 * Handles software updates using the Tauri updater plugin, behind the
 * Rust-side update gate (`updates_enabled`, see `src-tauri/src/updater/mod.rs`).
 *
 * Updates are disabled in Deino for now (PARITY_PLAN §8 D6). While the gate is
 * off, every check resolves to `{ enabled: false, available: false }` without
 * calling the updater plugin, which is not registered in that case.
 */

import { invoke } from '@tauri-apps/api/core';
import { check, Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

export interface UpdateInfo {
  /** False when this build has update checks turned off. */
  enabled: boolean;
  available: boolean;
  currentVersion: string;
  version?: string;
  date?: string;
  body?: string;
  downloadUrl?: string;
}

export interface UpdateProgress {
  downloaded: number;
  total: number;
  percentage: number;
}

/**
 * Update Service
 * Singleton service for managing app updates
 */
export class UpdateService {
  private updateCheckInProgress = false;
  private lastCheckTime: number | null = null;
  private enabledPromise: Promise<boolean> | null = null;
  private readonly CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000; // 24 hours

  /**
   * Whether this build allows update checks. The answer is fixed at build
   * time, so it is fetched once and cached.
   */
  isEnabled(): Promise<boolean> {
    this.enabledPromise ??= invoke<boolean>('updates_enabled').catch((error: unknown) => {
      console.error('Failed to read the update gate; treating updates as disabled:', error);
      return false;
    });
    return this.enabledPromise;
  }

  /**
   * Check for available updates
   * @param force Force check even if recently checked
   * @returns Promise with update information
   */
  async checkForUpdates(force = false): Promise<UpdateInfo> {
    if (!(await this.isEnabled())) {
      return { enabled: false, available: false, currentVersion: await getVersion() };
    }

    // Prevent concurrent update checks
    if (this.updateCheckInProgress) {
      throw new Error('Update check already in progress');
    }

    // Skip if checked recently (unless forced)
    if (!force && this.lastCheckTime) {
      const timeSinceLastCheck = Date.now() - this.lastCheckTime;
      if (timeSinceLastCheck < this.CHECK_INTERVAL_MS) {
        console.log('Skipping update check - checked recently');
        return {
          enabled: true,
          available: false,
          currentVersion: await getVersion(),
        };
      }
    }

    this.updateCheckInProgress = true;
    this.lastCheckTime = Date.now();

    try {
      const currentVersion = await getVersion();
      const update = await check();

      if (update?.available) {
        return {
          enabled: true,
          available: true,
          currentVersion,
          version: update.version,
          date: update.date,
          body: update.body,
        };
      }

      return {
        enabled: true,
        available: false,
        currentVersion,
      };
    } catch (error) {
      console.error('Failed to check for updates:', error);
      throw error;
    } finally {
      this.updateCheckInProgress = false;
    }
  }

  /**
   * Download and install the available update
   * @param update The update object from checkForUpdates
   * @param onProgress Optional progress callback
   * @returns Promise that resolves when download completes
   */
  async downloadAndInstall(
    update: Update,
    onProgress?: (progress: UpdateProgress) => void
  ): Promise<void> {
    try {
      // Download the update
      await update.download();

      // Notify progress if callback provided
      if (onProgress) {
        onProgress({ downloaded: 100, total: 100, percentage: 100 });
      }

      // Install and relaunch
      await update.install();
      await relaunch();
    } catch (error) {
      console.error('Failed to download/install update:', error);
      throw error;
    }
  }

  /**
   * Get the current app version
   * @returns Promise with version string
   */
  async getCurrentVersion(): Promise<string> {
    return getVersion();
  }

  /**
   * Check if an update check was performed recently
   * @returns true if checked within the interval
   */
  wasCheckedRecently(): boolean {
    if (!this.lastCheckTime) return false;
    const timeSinceLastCheck = Date.now() - this.lastCheckTime;
    return timeSinceLastCheck < this.CHECK_INTERVAL_MS;
  }
}

// Export singleton instance
export const updateService = new UpdateService();
