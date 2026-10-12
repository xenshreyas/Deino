/**
 * Resting states for the meeting slots whose feature cards have not shipped:
 * Chat (AI-02).
 * Each card replaces its entry in `slots.ts`.
 */
import { type Icon, ICON_PROPS_LARGE, MessageCircle } from '@/ui/icons';

import { meetingCopy } from '../copy';

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
