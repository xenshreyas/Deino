/**
 * Mock of `@/lib/ipc/generated/speakers`, over the shared mock speakers
 * (`mock-speaker-store.ts`).
 */
import type * as Generated from '@/lib/ipc/generated/speakers';

import { latency } from './latency';
import { ok } from './mock-database';
import { mockSpeakers as speakers } from './mock-speaker-store';

export const commands = {
  async participants(meetingId) {
    await latency();
    return ok(speakers.participants(meetingId));
  },
  async candidates(meetingId, query) {
    await latency();
    return ok(speakers.candidates(meetingId, query));
  },
  async rename(rename) {
    await latency();
    return speakers.rename(rename);
  },
  async merge(meetingId, from, into) {
    await latency();
    return speakers.merge(meetingId, from, into);
  },
  async confirmSuggestion(meetingId, speakerId) {
    await latency();
    return speakers.confirm(meetingId, speakerId);
  },
  async dismissSuggestion(speakerId) {
    await latency();
    return speakers.dismiss(speakerId);
  },
  async voiceProfiles() {
    await latency();
    return ok(speakers.voiceprints.profiles());
  },
  async deleteVoiceprint(id) {
    await latency();
    return speakers.voiceprints.delete(id);
  },
  async forgetVoice(personId) {
    await latency();
    return speakers.voiceprints.forget(personId);
  },
} satisfies typeof Generated.commands;
