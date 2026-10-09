import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { cn } from '@/ui/cn';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/ui/tabs';

import { meetingCopy } from '../copy';
import { MEETING_PANELS } from '../slots';
import type { MeetingPanelId } from '../types';

/** Props of {@link MeetingSidePanel}. */
export interface MeetingSidePanelProps {
  meetingId: MeetingId;
  open: boolean;
}

/**
 * The right panel (320px, toggled with `]`): one tab per registered section
 * (Chat, Participants, Details).
 */
export function MeetingSidePanel({ meetingId, open }: MeetingSidePanelProps) {
  const [section, setSection] = useState<string>(MEETING_PANELS[0]?.id ?? 'details');
  return (
    <aside
      aria-label={meetingCopy.panelLabel}
      className={cn(
        'h-full shrink-0 overflow-hidden border-l bg-bg transition-[width,border-color] duration-(--dur-slow) ease-out',
        open ? 'w-(--right-panel-width) border-border' : 'invisible w-0 border-transparent',
      )}
    >
      <Tabs
        value={section}
        onValueChange={setSection}
        className="flex h-full w-(--right-panel-width) flex-col"
      >
        <TabsList className="shrink-0 gap-4 px-4">
          {MEETING_PANELS.map((panel) => (
            <TabsTrigger key={panel.id} value={panel.id satisfies MeetingPanelId}>
              {panel.label}
            </TabsTrigger>
          ))}
        </TabsList>
        {MEETING_PANELS.map(({ id, Component }) => (
          <TabsContent key={id} value={id} className="min-h-0 flex-1 overflow-y-auto">
            <Component meetingId={meetingId} />
          </TabsContent>
        ))}
      </Tabs>
    </aside>
  );
}
