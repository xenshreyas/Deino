/**
 * An answer: its text runs, each followed by a numbered citation chip when
 * it cites the transcript, and a quiet Copy that leaves the chips out.
 */
import { toast } from 'sonner';

import type { MeetingId } from '@/features/meetings';
import { Copy } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { plainText } from '../answer-text';
import { askCopy } from '../copy';
import type { AnswerPart } from '../types';
import { CitationChip } from './citation-chip';

/** Props of {@link AnswerBody}. */
export interface AnswerBodyProps {
  meetingId: MeetingId;
  parts: readonly AnswerPart[];
}

/** The runs, their chips numbered in order. */
export function AnswerBody({ meetingId, parts }: AnswerBodyProps) {
  let cited = 0;
  const runs = parts.map((part, index) => {
    if (part.citations.length > 0) cited += 1;
    return { key: `${String(index)}-${part.text.slice(0, 12)}`, part, number: cited };
  });
  return (
    <div className="group/answer grid gap-1">
      <p className="text-sm leading-relaxed text-pretty whitespace-pre-wrap text-fg">
        {runs.map(({ key, part, number }) => (
          <span key={key}>
            {part.text}
            {part.citations.length > 0 ? (
              <CitationChip meetingId={meetingId} citations={part.citations} number={number} />
            ) : null}
          </span>
        ))}
      </p>
      <div className="flex opacity-0 transition-opacity duration-(--dur-fast) group-focus-within/answer:opacity-100 group-hover/answer:opacity-100">
        <IconButton
          icon={Copy}
          label={askCopy.copyAnswer}
          className="-ml-1.5 size-6"
          onClick={() => {
            void navigator.clipboard
              .writeText(plainText(parts))
              .then(() => toast.success(askCopy.copied));
          }}
        />
      </div>
    </div>
  );
}
