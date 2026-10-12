/**
 * Mock of `@/lib/ipc/generated/retention`: nothing has expired, unless the
 * URL asks to see a meeting whose retention ran: `?retention=transcript`
 * (the transcript and recording were deleted) or `?retention=audio` (only
 * the recording).
 *
 * It imports no feature code, so the feature reaching it through its
 * generated bindings forms no cycle.
 */
import type * as Generated from '@/lib/ipc/generated/retention';

import { ok } from './command-result';
import { latency } from './latency';

/** When the demo's retention ran. */
const DELETED_AT = '2026-10-01T06:00:00Z';

/** What `?retention=` says retention removed from every meeting. */
function demoStatus(): Generated.MeetingRetention {
  const asked = new URLSearchParams(window.location.search).get('retention');
  return {
    audioDeletedAt: asked === 'audio' || asked === 'transcript' ? DELETED_AT : null,
    transcriptDeletedAt: asked === 'transcript' ? DELETED_AT : null,
  };
}

export const commands = {
  async status() {
    await latency();
    return ok(demoStatus());
  },
} satisfies typeof Generated.commands;
