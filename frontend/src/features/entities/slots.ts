/**
 * The person and company pages' slot registry. Cards that own a slot fill
 * it by editing this file only, like the meeting page's `slots.ts`.
 *
 * | Slot | Today | Filled by |
 * |---|---|---|
 * | `ask` | nothing: hidden until it ships (UX-27); `AskSlotPlaceholder` is the resting bar | AI-02 (scoped chat) |
 * | `voice` | whether Deino knows the person's voice, and where to manage it | CAP-04 |
 */
import type { ComponentType } from 'react';

import { PersonVoice } from './components/voice-slot';
import type { CompanyId, PersonId } from './types';

/** Whom a scoped chat asks about. */
export type AskScope = { kind: 'person'; id: PersonId } | { kind: 'company'; id: CompanyId };

/** Props of the `ask` slot. */
export interface AskSlotProps {
  scope: AskScope;
  /** The person's or company's name, for the placeholder. */
  name: string;
}

/** Props of the `voice` slot (person pages only). */
export interface VoiceSlotProps {
  personId: PersonId;
}

/** The slots and what fills them today; `null` renders nothing. */
export const ENTITY_SLOTS: {
  ask: ComponentType<AskSlotProps> | null;
  voice: ComponentType<VoiceSlotProps>;
} = {
  ask: null,
  voice: PersonVoice,
};
