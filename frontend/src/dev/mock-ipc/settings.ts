/**
 * Mock of `@/lib/ipc/generated/settings`: the settings in memory, changed by
 * RFC 7396 merge patches, with `SettingsChanged` after every change.
 *
 * It starts past onboarding, so `pnpm dev:mock` opens the app. It does not
 * repeat the Rust validation: a patch the UI could not have typed is the
 * Rust contract tests' job.
 */
import type * as Generated from '@/lib/ipc/generated/settings';

import { latency } from './latency';
import { ok } from './command-result';
import { mockEvent } from './mock-event';
import { currentSettings, replaceSettings } from './settings-store';

export const events = {
  settingsChanged: mockEvent<Generated.SettingsChanged>('plugin:settings:settings-changed'),
} satisfies typeof Generated.events;

export const commands = {
  async get() {
    await latency();
    return ok(structuredClone(currentSettings()));
  },
  async patch(patch) {
    await latency();
    // The UI only sends typed `SettingsPatch` values, so the merge keeps the shape.
    const settings = mergePatch(currentSettings(), patch) as Generated.Settings;
    replaceSettings(settings);
    void events.settingsChanged.emit({ settings: structuredClone(settings) });
    return ok(structuredClone(settings));
  },
} satisfies typeof Generated.commands;

/** RFC 7396: objects merge key by key, anything else replaces the target. */
function mergePatch(target: unknown, patch: unknown): unknown {
  if (!isRecord(patch)) return patch;
  const merged: Record<string, unknown> = isRecord(target) ? { ...target } : {};
  for (const [key, value] of Object.entries(patch)) {
    merged[key] = value === null ? null : mergePatch(merged[key], value);
  }
  return merged;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
