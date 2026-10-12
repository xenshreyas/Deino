/**
 * A meeting's vocabulary corrections, last in its Details panel and
 * collapsed (UX-04): what changed ("Sreyas" → "Shreyas"), each with Undo,
 * and Undo all. Renders nothing when the vocabulary changed nothing.
 */
import { toast } from 'sonner';

import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/ui/accordion';
import { Button } from '@/ui/button';
import { IconButton } from '@/ui/icon-button';
import { ArrowRight, ICON_PROPS, Undo2 } from '@/ui/icons';

import { vocabularyCopy as copy } from '../copy';
import { changedWords } from '../entries';
import { useCorrections, useUndoCorrections } from '../queries';
import type { CorrectedSegment, MeetingId, SegmentId } from '../types';

/** Props of {@link MeetingCorrections}. */
export interface MeetingCorrectionsProps {
  meetingId: MeetingId;
}

/** Undoes corrections, toasting what happened. */
function useUndo(meetingId: MeetingId) {
  const undo = useUndoCorrections(meetingId);
  return (segmentId: SegmentId | null) => {
    undo.mutate(segmentId, {
      onSuccess: (report) => {
        if (report.restored > 0) toast.success(copy.undone(report.restored));
      },
      onError: (error) => toast.error(copy.undoFailed, { description: error.message }),
    });
  };
}

/** One correction: the words before and after, and its Undo. */
function CorrectionLine(props: { correction: CorrectedSegment; onUndo: () => void }) {
  const { from, to } = changedWords(props.correction.originalText, props.correction.correctedText);
  return (
    <li className="grid min-h-7 grid-cols-[1fr_auto] items-center gap-2 text-sm">
      <p className="min-w-0 py-1 break-words">
        <span className="text-fg-muted line-through decoration-fg-subtle">{from}</span>
        <ArrowRight
          {...ICON_PROPS}
          size={12}
          className="mx-1.5 inline align-[-1px] text-fg-subtle"
        />
        <span className="text-fg">{to}</span>
      </p>
      {props.correction.undoable ? (
        <IconButton icon={Undo2} label={copy.undoLine(from, to)} onClick={props.onUndo} />
      ) : (
        <span className="shrink-0 text-xs text-fg-subtle">{copy.edited}</span>
      )}
    </li>
  );
}

/** The meeting's corrections, or nothing. */
export function MeetingCorrections({ meetingId }: MeetingCorrectionsProps) {
  const corrections = useCorrections(meetingId);
  const undo = useUndo(meetingId);
  const list = corrections.data ?? [];
  if (list.length === 0) return null;
  return (
    <Accordion type="single" collapsible>
      <AccordionItem value="corrections" aria-label={copy.correctionsTitle}>
        <AccordionTrigger className="py-1 text-xs font-normal text-fg-muted">
          {copy.correctionsTitle} · {copy.correctionsCount(list.length)}
        </AccordionTrigger>
        <AccordionContent className="pb-0">
          <div className="flex justify-end">
            <Button
              variant="ghost"
              size="sm"
              disabled={!list.some((c) => c.undoable)}
              onClick={() => {
                undo(null);
              }}
            >
              {copy.undoAll}
            </Button>
          </div>
          <ul className="flex flex-col">
            {list.map((correction) => (
              <CorrectionLine
                key={correction.segmentId}
                correction={correction}
                onUndo={() => {
                  undo(correction.segmentId);
                }}
              />
            ))}
          </ul>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}
