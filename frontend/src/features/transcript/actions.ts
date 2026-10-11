/**
 * The transcript bubble's hover actions, as a registry (open/closed: a card
 * adds its action by replacing its entry here, nothing else changes).
 *
 * The toolbar shows the bubble's time, then every action whose `state` is
 * not `hidden`, in this order (ui.md Transcript bubble).
 *
 * | Action | Today | Filled by |
 * |---|---|---|
 * | Play from here | hidden | TRN-03 (seek the player) |
 * | Copy | the line's text to the clipboard (⌘C on a focused line) | TRN-01 |
 * | Clip | hidden | TRN-04 |
 * | Comment | hidden | TRN-06 |
 * | Delete | hidden | TRN-02 |
 */
import { toast } from 'sonner';

import { Copy, type Icon, MessageCircle, Play, Scissors, Trash2 } from '@/ui/icons';

import { transcriptCopy } from './copy';
import type { MeetingId, SpeakerView, TranscriptLine } from './types';

/** Every action a bubble can offer. */
export type TranscriptActionId = 'play' | 'copy' | 'clip' | 'comment' | 'delete';

/** Whether an action shows for a line, and whether it can run. */
export type TranscriptActionState = 'enabled' | 'disabled' | 'hidden';

/** What an action acts on. */
export interface TranscriptActionContext {
  meetingId: MeetingId;
  line: TranscriptLine;
  speaker: SpeakerView;
}

/** A key that runs the action on the focused line (`⌘C` is `{ key: 'c', meta: true }`). */
export interface TranscriptActionKey {
  key: string;
  meta: boolean;
}

/** One bubble action. */
export interface TranscriptAction {
  id: TranscriptActionId;
  label: string;
  icon: Icon;
  /** The key on a focused line, shown in the tooltip. */
  shortcut?: TranscriptActionKey & { display: string };
  /** `hidden` keeps an unbuilt action out of the toolbar. */
  state: (context: TranscriptActionContext) => TranscriptActionState;
  run: (context: TranscriptActionContext) => void | Promise<void>;
}

/** Hidden until its card lands. */
const hidden = (): TranscriptActionState => 'hidden';

/** Nothing to do yet. */
const noop = (): void => undefined;

/** Copies the line's text; a line still being spoken copies what is said so far. */
async function copyLine({ line }: TranscriptActionContext): Promise<void> {
  try {
    await navigator.clipboard.writeText(line.text);
    toast.success(transcriptCopy.copied);
  } catch {
    toast.error(transcriptCopy.copyFailed);
  }
}

/** The bubble actions, in toolbar order. */
export const TRANSCRIPT_ACTIONS: readonly TranscriptAction[] = [
  { id: 'play', label: transcriptCopy.playFromHere, icon: Play, state: hidden, run: noop },
  {
    id: 'copy',
    label: transcriptCopy.copyLine,
    icon: Copy,
    shortcut: { key: 'c', meta: true, display: '⌘C' },
    state: () => 'enabled',
    run: copyLine,
  },
  { id: 'clip', label: transcriptCopy.clip, icon: Scissors, state: hidden, run: noop },
  { id: 'comment', label: transcriptCopy.comment, icon: MessageCircle, state: hidden, run: noop },
  { id: 'delete', label: transcriptCopy.deleteLine, icon: Trash2, state: hidden, run: noop },
];

/** The action `event` triggers on a focused line, if any. */
export function actionForKey(
  actions: readonly TranscriptAction[],
  event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey'>,
): TranscriptAction | undefined {
  const meta = event.metaKey || event.ctrlKey;
  return actions.find(
    ({ shortcut }) =>
      shortcut !== undefined && shortcut.key === event.key.toLowerCase() && shortcut.meta === meta,
  );
}
