/**
 * Who said each line, from the meeting's participants (CAP-04): the same
 * names ("You" for an unnamed microphone, the person once named, else
 * "Speaker 2") and palette slots as the Participants panel.
 *
 * A saved line names its speaker. A live line only knows its channel until
 * it is saved, so it borrows the participant for that channel: the user's
 * own voice for the microphone, and the one other voice for system audio
 * while diarization has not split it yet.
 */
import { displayName, speakersCopy } from '@/features/speakers';

import { transcriptCopy } from '../copy';
import type { Participant, SpeakerId, SpeakerView, TranscriptLine } from '../types';

/** The recorder's slots: Me first, Them second (`PaletteColor::FIRST/SECOND`). */
const ME_COLOR = 1;
const THEM_COLOR = 2;

/** Stands in for the user before their speaker is stored. */
const FALLBACK_ME: SpeakerView = {
  key: 'channel:mic',
  speakerId: null,
  name: speakersCopy.you,
  isMe: true,
  color: ME_COLOR,
  unnamed: false,
};

/** Stands in for the other side before its speaker is stored. */
const FALLBACK_THEM: SpeakerView = {
  key: 'channel:system',
  speakerId: null,
  name: transcriptCopy.them,
  isMe: false,
  color: THEM_COLOR,
  unnamed: true,
};

/** A mixed (imported) line nobody has attributed. */
const FALLBACK_MIXED: SpeakerView = {
  key: 'channel:mixed',
  speakerId: null,
  name: transcriptCopy.unknownSpeaker,
  isMe: false,
  color: THEM_COLOR,
  unnamed: true,
};

/** The view of one participant. */
export function speakerView(participant: Participant): SpeakerView {
  return {
    key: participant.speakerId,
    speakerId: participant.speakerId,
    name: displayName(participant),
    isMe: participant.isMe,
    color: participant.color,
    unnamed: participant.person === null && !participant.isMe,
  };
}

/** Resolves the speaker of any line of one meeting. */
export class SpeakerDirectory {
  private readonly byId: ReadonlyMap<SpeakerId, SpeakerView>;
  private readonly me: SpeakerView;
  private readonly them: SpeakerView;

  constructor(participants: readonly Participant[]) {
    this.byId = new Map(participants.map((one) => [one.speakerId, speakerView(one)]));
    const views = [...this.byId.values()];
    const others = views.filter((view) => !view.isMe);
    this.me = views.find((view) => view.isMe) ?? FALLBACK_ME;
    this.them = others.length === 1 && others[0] !== undefined ? others[0] : FALLBACK_THEM;
  }

  /** Every participant, in the order they come (most speech first). */
  get all(): SpeakerView[] {
    return [...this.byId.values()];
  }

  /** Who said `line`. */
  of(line: TranscriptLine): SpeakerView {
    const stored = line.speakerId === null ? undefined : this.byId.get(line.speakerId);
    if (stored !== undefined) return stored;
    if (line.channel === 'mic') return this.me;
    return line.channel === 'system' ? this.them : FALLBACK_MIXED;
  }
}
