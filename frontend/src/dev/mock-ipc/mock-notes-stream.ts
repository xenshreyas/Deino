/**
 * Streamed notes for `pnpm dev:mock`: the notes the mock is about to save,
 * revealed a line at a time as `NotesRun` drafts, like Rust's runs (a
 * reading pause, then a finished line every few hundred ms), with Stop.
 */
import type {
  Block,
  MeetingId,
  doc as NoteDoc,
  NotesDraft,
  NotesPhase,
} from '@/lib/ipc/generated/enhance';

/** The pause before the first line, as a model reads the transcript. */
const READING_MS = 1200;
/** Between two lines. */
const LINE_MS = 260;

/** A list item of either kind. */
interface Item {
  content: Block[];
}

/** Lines in `blocks`: each heading, paragraph and list item. */
function countLines(blocks: readonly Block[]): number {
  return blocks.reduce((total, block) => {
    if (block.type === 'bulletList' || block.type === 'orderedList' || block.type === 'taskList') {
      const items: readonly Item[] = block.content;
      return total + items.reduce((sum, item) => sum + 1 + countLines(item.content.slice(1)), 0);
    }
    return total + 1;
  }, 0);
}

/** The first `budget.left` lines of `blocks`. */
function take(blocks: readonly Block[], budget: { left: number }): Block[] {
  const out: Block[] = [];
  for (const block of blocks) {
    if (budget.left <= 0) break;
    if (block.type === 'bulletList' || block.type === 'orderedList') {
      out.push({ ...block, content: takeItems(block.content, budget) });
    } else if (block.type === 'taskList') {
      out.push({ ...block, content: takeItems(block.content, budget) });
    } else {
      budget.left -= 1;
      out.push(block);
    }
  }
  return out;
}

function takeItems<T extends Item>(items: readonly T[], budget: { left: number }): T[] {
  const out: T[] = [];
  for (const item of items) {
    if (budget.left <= 0) break;
    budget.left -= 1;
    const [first, ...rest] = item.content;
    out.push({ ...item, content: first === undefined ? [] : [first, ...take(rest, budget)] });
  }
  return out;
}

/** The draft of `doc`'s first `lines` lines. */
export function draftOf(
  meetingId: MeetingId,
  doc: NoteDoc,
  lines: number,
  phase: NotesPhase,
): NotesDraft {
  return {
    meetingId,
    phase,
    doc: { type: 'doc', content: take(doc.content, { left: lines }) },
    lines,
    stale: false,
  };
}

/**
 * Reveals `doc` through `onDraft`, a line at a time; resolves `false` as
 * soon as `stopped()` says the user stopped it.
 */
export async function streamDoc(
  meetingId: MeetingId,
  doc: NoteDoc,
  onDraft: (draft: NotesDraft) => void,
  stopped: () => boolean,
): Promise<boolean> {
  const total = countLines(doc.content);
  onDraft(draftOf(meetingId, doc, 0, { kind: 'reading' }));
  await wait(READING_MS);
  for (let lines = 1; lines <= total; lines += 1) {
    if (stopped()) return false;
    onDraft(draftOf(meetingId, doc, lines, { kind: 'reading' }));
    await wait(LINE_MS);
  }
  if (stopped()) return false;
  onDraft(draftOf(meetingId, doc, total, { kind: 'saving' }));
  return true;
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
