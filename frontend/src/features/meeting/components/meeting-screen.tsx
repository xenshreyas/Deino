/**
 * The meeting page skeleton (PARITY_PLAN §4.6, ui.md §4.2): a 720px column
 * with the header and the Notes / Transcript / Insights / Clips tabs, the
 * right panel (Chat / Participants / Details) and the bottom player. Every
 * region is a slot in `slots.ts`; the legacy session publishes the Notes and
 * Transcript panels until their cards rebuild them.
 */
import { Link } from '@tanstack/react-router';

import { ShareMenu } from '@/features/export';
import { asMeetingId, type MeetingId } from '@/features/meetings';
import { EnhanceButton } from '@/features/notes';
import { TitleBarActions, useShellLayout } from '@/features/shell';
import { buttonVariants } from '@/ui/button';
import { ChevronRight, ICON_PROPS, PanelRight } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/ui/tabs';

import { meetingCopy } from '../copy';
import { useTabShortcuts } from '../hooks/use-tab-shortcuts';
import { LegacyMeetingSession } from '../legacy/meeting-session';
import type { MeetingSource, MeetingTabId } from '../search';
import type { MeetingMoment } from '../types';
import type { TranscriptMoment } from '@/features/transcript';
import { MEETING_PLAYER, MEETING_TABS } from '../slots';
import { MeetingHeader } from './meeting-header';
import { MeetingSidePanel } from './meeting-side-panel';

/** Props of {@link MeetingScreen}. */
export interface MeetingScreenProps {
  meetingId: string;
  source?: MeetingSource | undefined;
  tab: MeetingTabId;
  onTabChange: (tab: MeetingTabId) => void;
  /** A moment to show on the Transcript tab (`?t=` or `?segment=`). */
  moment?: TranscriptMoment | undefined;
  /** Called once the moment has been shown. */
  onMomentShown?: (() => void) | undefined;
}

/** "‹ Meetings" on the left of the title bar; Enhance, Share and the panel toggle on the right. */
function MeetingTitleBar({ meetingId }: { meetingId: MeetingId }) {
  const toggleRightPanel = useShellLayout((state) => state.toggleRightPanel);
  return (
    <TitleBarActions>
      <Link
        to="/meetings"
        className={buttonVariants({
          variant: 'ghost',
          size: 'sm',
          className: '-ml-1 gap-1',
        })}
      >
        <ChevronRight {...ICON_PROPS} className="rotate-180" />
        {meetingCopy.back}
      </Link>
      <div data-tauri-drag-region className="h-full flex-1" />
      <EnhanceButton meetingId={meetingId} />
      <ShareMenu meetingId={meetingId} />
      <IconButton
        icon={PanelRight}
        label={meetingCopy.togglePanel}
        shortcut="]"
        onClick={toggleRightPanel}
      />
    </TitleBarActions>
  );
}

/** The tab row and the selected tab's slot. */
function MeetingTabs({
  meetingId,
  tab,
  onTabChange,
  moment,
}: { meetingId: MeetingId; moment: MeetingMoment | undefined } & Pick<
  MeetingScreenProps,
  'tab' | 'onTabChange'
>) {
  const select = (value: string) => {
    const next = MEETING_TABS.find((candidate) => candidate.id === value);
    if (next !== undefined) onTabChange(next.id);
  };
  return (
    <Tabs value={tab} onValueChange={select} className="flex min-h-0 flex-1 flex-col">
      <TabsList data-chrome aria-label={meetingCopy.tabsLabel} className="mt-4 shrink-0">
        {MEETING_TABS.map((slot) => (
          <TabsTrigger key={slot.id} value={slot.id} title={`${slot.label} ${slot.shortcut}`}>
            {slot.label}
          </TabsTrigger>
        ))}
      </TabsList>
      {MEETING_TABS.map(({ id, Component }) => (
        <TabsContent key={id} value={id} className="min-h-0 flex-1">
          <Component meetingId={meetingId} moment={moment} />
        </TabsContent>
      ))}
    </Tabs>
  );
}

/** The meeting page. */
export function MeetingScreen({
  meetingId,
  source,
  tab,
  onTabChange,
  moment,
  onMomentShown,
}: MeetingScreenProps) {
  const id = asMeetingId(meetingId);
  const shown =
    moment === undefined
      ? undefined
      : { target: moment, onShown: onMomentShown ?? (() => undefined) };
  const rightPanelOpen = useShellLayout((state) => state.rightPanelOpen);
  useTabShortcuts(onTabChange);
  const Player = MEETING_PLAYER;
  return (
    <LegacyMeetingSession key={meetingId} meetingId={meetingId} source={source}>
      <MeetingTitleBar meetingId={id} />
      <div className="flex h-full min-w-0">
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="mx-auto flex min-h-0 w-full max-w-[720px] flex-1 flex-col px-6">
            <MeetingHeader meetingId={id} />
            <MeetingTabs meetingId={id} tab={tab} onTabChange={onTabChange} moment={shown} />
          </div>
          <Player meetingId={id} />
        </div>
        <MeetingSidePanel meetingId={id} open={rightPanelOpen} />
      </div>
    </LegacyMeetingSession>
  );
}
