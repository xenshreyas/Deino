/**
 * Mock of `@/lib/ipc/generated/consent`: the reminder setting, read from and
 * turned off in the mock settings store, so "Don't show again" in the record
 * panel shows up in Settings → Consent under `pnpm dev:mock`.
 *
 * It imports no feature code: the feature reaches this module through its
 * generated bindings, so importing it back would form a cycle.
 */
import type * as Generated from '@/lib/ipc/generated/consent';

import { ok } from './command-result';
import { latency } from './latency';
import { currentSettings, replaceSettings } from './settings-store';

export const commands = {
  async reminder() {
    await latency();
    return ok(currentSettings().consent.reminder);
  },
  async stopReminding() {
    await latency();
    const settings = currentSettings();
    replaceSettings({ ...settings, consent: { ...settings.consent, reminder: false } });
    return ok(null);
  },
} satisfies typeof Generated.commands;
