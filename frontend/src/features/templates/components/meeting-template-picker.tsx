import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { Check, ChevronDown, ICON_PROPS, LayoutTemplate, RotateCcw } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Skeleton } from '@/ui/skeleton';

import { reasonText, templatesCopy } from '../copy';
import { useMeetingTemplate, useSetMeetingTemplate, useTemplates } from '../queries';
import type { Template, TemplateChoice, TemplateId } from '../types';

/** Props of {@link MeetingTemplatePicker}. */
export interface MeetingTemplatePickerProps {
  meetingId: MeetingId;
  /**
   * Regenerates the meeting's notes with its current template, or `null`
   * when there are no notes yet (switching then needs no prompt).
   */
  onRegenerate: (() => void) | null;
}

/**
 * The Notes tab's template line: which template the meeting uses and why
 * ("Matched rule: external discovery call"), a menu to switch it, and after
 * a switch, a prompt to regenerate notes written with the old one.
 */
export function MeetingTemplatePicker({ meetingId, onRegenerate }: MeetingTemplatePickerProps) {
  const choice = useMeetingTemplate(meetingId);
  const set = useSetMeetingTemplate(meetingId);
  const [switched, setSwitched] = useState<string | null>(null);
  if (choice.data === undefined) return <Skeleton className="mt-4 h-5 w-56" />;
  const pick = (template: TemplateId | null) => {
    set.mutate(template, {
      onSuccess: (next) => {
        setSwitched(next.template.content.name);
      },
    });
  };
  return (
    <div className="flex flex-col gap-2 pt-4">
      <div className="flex min-w-0 items-center gap-2 text-xs text-fg-muted">
        <LayoutTemplate {...ICON_PROPS} className="shrink-0" />
        <TemplateMenu choice={choice.data} onPick={pick} />
        <span className="truncate">{reasonText(choice.data.reason)}</span>
      </div>
      {switched !== null && onRegenerate !== null ? (
        <RegeneratePrompt
          name={switched}
          onRegenerate={() => {
            setSwitched(null);
            onRegenerate();
          }}
          onDismiss={() => {
            setSwitched(null);
          }}
        />
      ) : null}
    </div>
  );
}

/** The current template's name, opening every template to choose from. */
function TemplateMenu({
  choice,
  onPick,
}: {
  choice: TemplateChoice;
  onPick: (id: TemplateId | null) => void;
}) {
  const templates = useTemplates();
  const all = templates.data ?? [];
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={templatesCopy.pickerLabel}
        className="flex h-6 items-center gap-1 rounded-sm px-1 text-sm text-fg transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        {choice.template.content.name}
        <ChevronDown {...ICON_PROPS} className="text-fg-muted" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-[360px] overflow-y-auto">
        {choice.reason.kind === 'chosen' ? <BackToAuto onPick={onPick} /> : null}
        <MenuGroup
          label={templatesCopy.yours}
          templates={all.filter((t) => !t.isBuiltin)}
          current={choice.template.id}
          onPick={onPick}
        />
        <MenuGroup
          label={templatesCopy.builtIn}
          templates={all.filter((t) => t.isBuiltin)}
          current={choice.template.id}
          onPick={onPick}
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Clears the meeting's choice, so selection is automatic again. */
function BackToAuto({ onPick }: { onPick: (id: null) => void }) {
  return (
    <>
      <DropdownMenuItem
        onSelect={() => {
          onPick(null);
        }}
      >
        <RotateCcw {...ICON_PROPS} />
        {templatesCopy.pickerAuto}
      </DropdownMenuItem>
      <DropdownMenuSeparator />
    </>
  );
}

function MenuGroup({
  label,
  templates,
  current,
  onPick,
}: {
  label: string;
  templates: Template[];
  current: TemplateId;
  onPick: (id: TemplateId) => void;
}) {
  if (templates.length === 0) return null;
  return (
    <>
      <DropdownMenuLabel>{label}</DropdownMenuLabel>
      {templates.map((template) => (
        <DropdownMenuItem
          key={template.id}
          onSelect={() => {
            onPick(template.id);
          }}
        >
          <Check {...ICON_PROPS} className={cn(template.id !== current && 'invisible')} />
          {template.content.name}
        </DropdownMenuItem>
      ))}
    </>
  );
}

/** "Notes were written before you chose X." with Regenerate and Not now. */
function RegeneratePrompt({
  name,
  onRegenerate,
  onDismiss,
}: {
  name: string;
  onRegenerate: () => void;
  onDismiss: () => void;
}) {
  return (
    <div
      role="status"
      className="flex items-center gap-3 rounded-lg bg-accent-soft px-3 py-2 text-sm text-fg"
    >
      <span className="min-w-0 flex-1">{templatesCopy.regenerateTitle(name)}</span>
      <Button variant="ghost" size="sm" onClick={onDismiss}>
        {templatesCopy.dismiss}
      </Button>
      <Button size="sm" onClick={onRegenerate}>
        {templatesCopy.regenerate}
      </Button>
    </div>
  );
}
