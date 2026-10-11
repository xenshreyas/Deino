/**
 * Who said each line. A saved line names its speaker; a live line only
 * knows its channel until it is saved, so it borrows the meeting's speaker
 * for that channel: the user's own voice for the microphone, and the one
 * other voice for system audio while diarization has not split it yet.
 */
import { transcriptCopy } from '../copy';
import type {
  PaletteColor,
  SpeakerId,
  SpeakerView,
  TranscriptLine,
  TranscriptSpeaker,
} from '../types';

/** The recorder's slots: Me first, Them second (`PaletteColor::FIRST/SECOND`). */
const ME_COLOR: PaletteColor = 1;
const THEM_COLOR: PaletteColor = 2;

/** Stands in for the user before a Me speaker is stored. */
const FALLBACK_ME: SpeakerView = {
  key: 'channel:mic',
  speakerId: null,
  name: transcriptCopy.me,
  isMe: true,
  color: ME_COLOR,
  identified: true,
};

/** Stands in for the other side before a Them speaker is stored. */
const FALLBACK_THEM: SpeakerView = {
  key: 'channel:system',
  speakerId: null,
  name: transcriptCopy.them,
  isMe: false,
  color: THEM_COLOR,
  identified: false,
};

/** A mixed (imported) line nobody has attributed. */
const FALLBACK_MIXED: SpeakerView = {
  key: 'channel:mixed',
  speakerId: null,
  name: transcriptCopy.unknownSpeaker,
  isMe: false,
  color: THEM_COLOR,
  identified: false,
};

/** The view of a stored speaker. */
export function speakerView(speaker: TranscriptSpeaker): SpeakerView {
  return {
    key: speaker.id,
    speakerId: speaker.id,
    name: speaker.name,
    isMe: speaker.isMe,
    color: speaker.color,
    identified: speaker.isMe || speaker.personId !== null,
  };
}

/** Resolves the speaker of any line of one meeting. */
export class SpeakerDirectory {
  private readonly byId: ReadonlyMap<SpeakerId, SpeakerView>;
  private readonly me: SpeakerView;
  private readonly them: SpeakerView;

  constructor(speakers: readonly TranscriptSpeaker[]) {
    this.byId = new Map(speakers.map((speaker) => [speaker.id, speakerView(speaker)]));
    const views = [...this.byId.values()];
    const others = views.filter((view) => !view.isMe);
    this.me = views.find((view) => view.isMe) ?? FALLBACK_ME;
    this.them = others.length === 1 && others[0] !== undefined ? others[0] : FALLBACK_THEM;
  }

  /** Every stored speaker, in the order Rust serves them (by label). */
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

/** Text colour per palette slot; literal so Tailwind sees every class. */
const TEXT_CLASS: Readonly<Record<number, string>> = {
  1: 'text-speaker-1',
  2: 'text-speaker-2',
  3: 'text-speaker-3',
  4: 'text-speaker-4',
  5: 'text-speaker-5',
  6: 'text-speaker-6',
};

/** Avatar fill per palette slot: the speaker colour at 16% (ui.md Speaker chip). */
const TINT_CLASS: Readonly<Record<number, string>> = {
  1: 'bg-speaker-1/16',
  2: 'bg-speaker-2/16',
  3: 'bg-speaker-3/16',
  4: 'bg-speaker-4/16',
  5: 'bg-speaker-5/16',
  6: 'bg-speaker-6/16',
};

/** The text class of a speaker colour (slot 1 for anything out of range). */
export function speakerTextClass(color: PaletteColor): string {
  return TEXT_CLASS[color] ?? 'text-speaker-1';
}

/** The avatar tint class of a speaker colour. */
export function speakerTintClass(color: PaletteColor): string {
  return TINT_CLASS[color] ?? 'bg-speaker-1/16';
}

/** Splits text into user-perceived characters, so an initial is never half an emoji. */
const graphemes = new Intl.Segmenter(undefined, { granularity: 'grapheme' });

/** The first character of `word`, upper-cased. */
function initial(word: string): string {
  const [first] = graphemes.segment(word);
  return first?.segment.toLocaleUpperCase() ?? '';
}

/** One or two initials of `name` for the avatar ("Priya Shah" → "PS"). */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/u).filter(Boolean);
  const first = words[0] ?? '';
  const last = words.length > 1 ? (words[words.length - 1] ?? '') : '';
  return initial(first) + initial(last);
}
