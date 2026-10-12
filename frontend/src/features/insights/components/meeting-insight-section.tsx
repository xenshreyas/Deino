/**
 * One insight on the meeting's Insights tab: its name and state, then each
 * answer with who said it and when. Answers stay readable while a new run
 * is pending; a stale or failed run offers "Run again".
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { CircleAlert, ICON_PROPS, RotateCcw } from '@/ui/icons';

import { insightsCopy } from '../copy';
import type { MeetingId, MeetingInsight } from '../types';
import { AnswerValue, Attribution } from './answer-view';

const copy = insightsCopy.meeting;

/** Props of {@link MeetingInsightSection}. */
export interface MeetingInsightSectionProps {
  entry: MeetingInsight;
  meetingId: MeetingId;
  onRerun: () => void;
}

/** The heading row, the state line and the answers. */
export function MeetingInsightSection({ entry, meetingId, onRerun }: MeetingInsightSectionProps) {
  const { def, run, stale } = entry;
  const pending = run.status === 'pending';
  const headingId = `insight-${def.id}`;
  return (
    <section aria-labelledby={headingId} aria-busy={pending}>
      <div className="flex items-baseline justify-between gap-3">
        <h3 id={headingId} className="text-base font-semibold text-fg">
          {def.name}
        </h3>
        <State entry={entry} onRerun={onRerun} />
      </div>
      {run.answers.length === 0 ? (
        run.status === 'done' ? (
          <p className="mt-2 text-sm text-fg-muted">{copy.nothing}</p>
        ) : null
      ) : (
        <ol className={cn('mt-3 grid gap-4', (pending || stale) && 'opacity-70')}>
          {run.answers.map((answer, index) => (
            // Answers are a write-once list; their order is their identity.
            <li key={index} className="grid gap-1.5">
              <AnswerValue value={answer.value} />
              <Attribution meetingId={meetingId} answer={answer} className="justify-self-start" />
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

/** Running, failed (with why), stale, or nothing to say. */
function State({ entry, onRerun }: { entry: MeetingInsight; onRerun: () => void }) {
  const { run, stale } = entry;
  if (run.status === 'pending') {
    return <span className="animate-pulse-soft text-xs text-fg-muted">{copy.running}</span>;
  }
  const failed = run.status === 'failed';
  if (!failed && !stale) return null;
  return (
    <div className="flex min-w-0 items-center gap-2">
      <span
        className={cn(
          'flex min-w-0 items-center gap-1 text-xs',
          failed ? 'text-danger' : 'text-fg-muted',
        )}
      >
        {failed ? <CircleAlert {...ICON_PROPS} size={14} className="shrink-0" /> : null}
        <span className="truncate">{failed ? run.error : insightsCopy.table.stale}</span>
      </span>
      <Button variant="ghost" size="sm" onClick={onRerun}>
        <RotateCcw {...ICON_PROPS} />
        {copy.rerun}
      </Button>
    </div>
  );
}
