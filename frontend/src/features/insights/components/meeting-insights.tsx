/**
 * A meeting's insights (AI-06, UX-03): {@link MeetingInsights} is the
 * section that ends the Notes tab, each insight that ran here with its
 * answers attributed to the moment they were said, and "Run again" for a
 * stale or failed one. It shows nothing until an insight has run here.
 * {@link MeetingInsightsSubmenu} is the meeting ⋯ menu's "Run insight",
 * which runs any insight on this meeting.
 */
import { toast } from 'sonner';

import { Lightbulb, ICON_PROPS, Play, RotateCcw } from '@/ui/icons';
import {
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from '@/ui/menu';

import { insightsCopy } from '../copy';
import { useMeetingInsights, useRunInsight } from '../queries';
import type { InsightDef, InsightId, MeetingId, MeetingInsight } from '../types';
import { MeetingInsightSection } from './meeting-insight-section';

const copy = insightsCopy.meeting;

/** Props of {@link MeetingInsights}. */
export interface MeetingInsightsProps {
  meetingId: MeetingId;
}

/** Runs one insight on this meeting, toasting a failure to queue it. */
export function useRunHere(meetingId: MeetingId) {
  const run = useRunInsight();
  return {
    pending: run.isPending,
    run: (id: InsightId) => {
      run.mutate(
        { id, meetingIds: [meetingId] },
        { onError: (error) => toast.error(error.message) },
      );
    },
  };
}

/** The insights that ran on this meeting, or nothing. */
export function MeetingInsights({ meetingId }: MeetingInsightsProps) {
  const insights = useMeetingInsights(meetingId);
  const here = useRunHere(meetingId);
  const entries = insights.data?.entries ?? [];
  if (entries.length === 0) return null;
  return (
    <section aria-label={copy.heading} className="flex flex-col gap-4 border-t border-border pt-6">
      <h2 className="text-lg font-semibold text-fg">{copy.heading}</h2>
      <div className="grid gap-10">
        {entries.map((entry: MeetingInsight) => (
          <MeetingInsightSection
            key={entry.def.id}
            entry={entry}
            meetingId={meetingId}
            onRerun={() => {
              here.run(entry.def.id);
            }}
          />
        ))}
      </div>
    </section>
  );
}

/** One insight in the submenu: Run, or Run again when it already ran here. */
function RunItem(props: { def: InsightDef; ran: boolean; disabled: boolean; onRun: () => void }) {
  const ItemIcon = props.ran ? RotateCcw : Play;
  return (
    <DropdownMenuItem disabled={props.disabled} onSelect={props.onRun}>
      <ItemIcon {...ICON_PROPS} />
      <span className="truncate">{props.def.name}</span>
    </DropdownMenuItem>
  );
}

/** "Run insight ›": every insight, the ones not run here first. */
export function MeetingInsightsSubmenu({ meetingId }: MeetingInsightsProps) {
  const insights = useMeetingInsights(meetingId);
  const here = useRunHere(meetingId);
  const choices = [
    ...(insights.data?.others ?? []).map((def) => ({ def, ran: false })),
    ...(insights.data?.entries ?? []).map(({ def }) => ({ def, ran: true })),
  ];
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Lightbulb {...ICON_PROPS} />
        {copy.runOne}
      </DropdownMenuSubTrigger>
      <DropdownMenuSubContent className="max-h-[360px] w-64 overflow-y-auto">
        {choices.length === 0 ? <DropdownMenuLabel>{copy.none}</DropdownMenuLabel> : null}
        {choices.map(({ def, ran }) => (
          <RunItem
            key={def.id}
            def={def}
            ran={ran}
            disabled={here.pending}
            onRun={() => {
              here.run(def.id);
            }}
          />
        ))}
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}
