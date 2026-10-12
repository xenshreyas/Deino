/**
 * The transcript bubble's hover actions, as a registry (open/closed: a card
 * adds its action by replacing its entry here, nothing else changes).
 *
 * The toolbar shows the bubble's time, then every action whose `state` is
 * not `hidden`, in this order (ui.md Transcript bubble); when the space
 * beside the bubble runs out, the `rare` ones move into a ⋯ menu.
 *
 * | Action | Today | Filled by |
 * |---|---|---|
 * | Play from here | seeks the player there and plays (saved lines, with a recording) | TRN-03 |
 * | Copy | the line's text to the clipboard (⌘C on a focused line) | TRN-01 |
 * | Edit | the text in place (↵ on a focused line; or double-click) | TRN-02 |
 * | Clip | hidden | TRN-04 |
 * | Comment | hidden | TRN-06 |
 * | Delete | soft delete with Undo (⌘⌫ on a focused line) | TRN-02 |
 */
import type { QueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import { playerActions } from '@/features/player';
import { Copy, type Icon, MessageCircle, Pencil, Play, Scissors, Trash2 } from '@/ui/icons';

import { transcriptCopy } from './copy';
import { startEditing } from './edit/editing-store';
import { deleteLine } from './edit/line-edits';
import type { MeetingId, SpeakerView, TranscriptLine } from './types';

/** Every action a bubble can offer. */
export type TranscriptActionId = 'play' | 'copy' | 'edit' | 'clip' | 'comment' | 'delete';

/** Whether an action shows for a line, and whether it can run. */
export type TranscriptActionState = 'enabled' | 'disabled' | 'hidden';

/** What an action acts on. */
export interface TranscriptActionContext {
  meetingId: MeetingId;
  line: TranscriptLine;
  speaker: SpeakerView;
  /** The app's query client, for actions that write. */
  client: QueryClient;
  /** The meeting's recording is in the player. */
  playable: boolean;
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
  /** Goes into the ⋯ menu first when the toolbar has no room. */
  rare?: true;
  /** `hidden` keeps an unbuilt action out of the toolbar. */
  state: (context: TranscriptActionContext) => TranscriptActionState;
  run: (context: TranscriptActionContext) => void | Promise<void>;
}

/** Hidden until its card lands. */
const hidden = (): TranscriptActionState => 'hidden';

/** Nothing to do yet. */
const noop = (): void => undefined;

/** Only saved lines can be played from, edited or deleted. */
const whenSaved = ({ line }: TranscriptActionContext): TranscriptActionState =>
  line.open ? 'hidden' : 'enabled';

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
  {
    id: 'play',
    label: transcriptCopy.playFromHere,
    icon: Play,
    state: (context) => (context.playable ? whenSaved(context) : 'hidden'),
    run: ({ meetingId, line }) => {
      playerActions.playFrom(meetingId, line.start);
    },
  },
  {
    id: 'copy',
    label: transcriptCopy.copyLine,
    icon: Copy,
    shortcut: { key: 'c', meta: true, display: '⌘C' },
    state: () => 'enabled',
    run: copyLine,
  },
  {
    id: 'edit',
    label: transcriptCopy.editLine,
    icon: Pencil,
    shortcut: { key: 'enter', meta: false, display: '↵' },
    rare: true,
    state: whenSaved,
    run: ({ line }) => {
      startEditing(line.id);
    },
  },
  { id: 'clip', label: transcriptCopy.clip, icon: Scissors, rare: true, state: hidden, run: noop },
  {
    id: 'comment',
    label: transcriptCopy.comment,
    icon: MessageCircle,
    rare: true,
    state: hidden,
    run: noop,
  },
  {
    id: 'delete',
    label: transcriptCopy.deleteLine,
    icon: Trash2,
    shortcut: { key: 'backspace', meta: true, display: '⌘⌫' },
    state: whenSaved,
    run: ({ client, meetingId, line }) => deleteLine({ client, meetingId, segmentId: line.id }),
  },
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
