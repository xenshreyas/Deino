/**
 * One insight answer, as the meeting's Insights tab shows it: the value
 * (prose, or a small grid of typed fields) and who said it when, linking to
 * that moment of the transcript.
 */
import { Link } from '@tanstack/react-router';

import { cn } from '@/ui/cn';
import { ArrowUpRight, ICON_PROPS } from '@/ui/icons';

import { insightsCopy } from '../copy';
import { formatMoment, formatValue } from '../model';
import type { InsightAnswer, InsightValue, MeetingId } from '../types';

/** The answer's value: text as prose, fields as label and value pairs. */
export function AnswerValue({ value }: { value: InsightValue }) {
  if (value.kind === 'text') {
    return <p className="text-sm/6 whitespace-pre-line text-fg">{value.text}</p>;
  }
  const filled = value.fields.filter((entry) => entry.value !== null);
  return (
    <dl className="grid grid-cols-[minmax(6rem,max-content)_1fr] gap-x-4 gap-y-1.5 text-sm/6">
      {filled.map((entry) => (
        <div key={entry.name} className="contents">
          <dt className="text-fg-muted">{entry.name}</dt>
          <dd className="min-w-0 text-fg tabular-nums">{formatValue(entry.value)}</dd>
        </div>
      ))}
    </dl>
  );
}

/** Props of {@link Attribution}. */
export interface AttributionProps {
  meetingId: MeetingId;
  answer: InsightAnswer;
  className?: string;
}

/** "Maya Chen · 04:31", opening the transcript at that moment. */
export function Attribution({ meetingId, answer, className }: AttributionProps) {
  const at = answer.timestampMs;
  const who = answer.speaker ?? insightsCopy.table.unattributed;
  if (at === null) {
    return <span className={cn('text-xs text-fg-subtle', className)}>{who}</span>;
  }
  const moment = formatMoment(at);
  return (
    <Link
      to="/meetings/$meetingId"
      params={{ meetingId }}
      search={{ tab: 'transcript', t: at }}
      aria-label={insightsCopy.meeting.openMoment(who, moment)}
      onClick={(event) => {
        event.stopPropagation();
      }}
      className={cn(
        'group/moment inline-flex items-center gap-1 rounded-xs text-xs whitespace-nowrap text-fg-muted',
        'underline-offset-4 transition-colors duration-(--dur-fast) ease-out hover:text-fg hover:underline',
        'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
        className,
      )}
    >
      <span className="max-w-40 truncate">{who}</span>
      <span aria-hidden className="text-fg-subtle">
        ·
      </span>
      <span className="tabular-nums">{moment}</span>
      <ArrowUpRight
        {...ICON_PROPS}
        size={12}
        className="opacity-0 transition-opacity duration-(--dur-fast) group-hover/moment:opacity-100"
      />
    </Link>
  );
}
