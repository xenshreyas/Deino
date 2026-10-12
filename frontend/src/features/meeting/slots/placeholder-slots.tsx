/**
 * Resting states for the meeting slots whose feature cards have not shipped:
 * Clips (TRN-04) and Chat (AI-02).
 * Each card replaces its entry in `slots.ts`.
 */
import { EmptyState } from '@/ui/empty-state';
import { type Icon, ICON_PROPS_LARGE, MessageCircle, Play } from '@/ui/icons';

import { meetingCopy } from '../copy';

/** A tab-sized empty state. */
function TabPlaceholder({ icon, title, body }: { icon: Icon; title: string; body: string }) {
  return <EmptyState className="pt-[12vh]" icon={icon} title={title} description={body} />;
}

/** A compact empty state for the right panel. */
function PanelPlaceholder({
  icon: PanelIcon,
  title,
  body,
}: {
  icon: Icon;
  title: string;
  body: string;
}) {
  return (
    <div className="flex flex-col items-start gap-2 px-4 py-6">
      <div className="flex size-9 items-center justify-center rounded-lg bg-bg-muted text-fg-muted">
        <PanelIcon {...ICON_PROPS_LARGE} size={18} />
      </div>
      <h3 className="pt-1 text-sm font-semibold text-fg">{title}</h3>
      <p className="text-sm text-pretty text-fg-muted">{body}</p>
    </div>
  );
}

/** Clips tab. */
export function ClipsSlot() {
  return <TabPlaceholder icon={Play} title={meetingCopy.clipsTitle} body={meetingCopy.clipsBody} />;
}

/** Chat panel. */
export function ChatPanelSlot() {
  return (
    <PanelPlaceholder
      icon={MessageCircle}
      title={meetingCopy.chatTitle}
      body={meetingCopy.chatBody}
    />
  );
}
