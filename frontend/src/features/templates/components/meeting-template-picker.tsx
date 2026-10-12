/**
 * The meeting's template menu (NOT-04, UX-06, UX-07): a quiet button with
 * the template's name, in the notes' toolbar beside Enhanced / My notes.
 * Why Deino chose it ("Matched rule: external discovery call") is the
 * menu's header, not a line on the page. After a switch, the notes page
 * shows {@link TemplateSwitchedPrompt} to regenerate notes written with
 * the old template.
 */
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

import { reasonText, templatesCopy } from '../copy';
import { useMeetingTemplate, useSetMeetingTemplate, useTemplates } from '../queries';
import type { Template, TemplateChoice, TemplateId } from '../types';

/** Props of {@link MeetingTemplateMenu}. */
export interface MeetingTemplateMenuProps {
  meetingId: MeetingId;
  /** Hears the new template's name once a switch is saved. */
  onSwitched: (name: string) => void;
}

/** The template button and its menu; nothing until the meeting's template has loaded. */
export function MeetingTemplateMenu({ meetingId, onSwitched }: MeetingTemplateMenuProps) {
  const choice = useMeetingTemplate(meetingId);
  const set = useSetMeetingTemplate(meetingId);
  if (choice.data === undefined) return null;
  const pick = (template: TemplateId | null) => {
    set.mutate(template, {
      onSuccess: (next) => {
        onSwitched(next.template.content.name);
      },
    });
  };
  return <TemplateMenu choice={choice.data} onPick={pick} />;
}

/** The current template's name, opening every template to choose from. */
function TemplateMenu({
  choice,
  onPick,
}: {
  choice: TemplateChoice;
  onPick: (id: TemplateId | null) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={templatesCopy.pickerLabel}
        className="flex h-7 max-w-48 items-center gap-1.5 rounded-sm px-1.5 text-xs text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <LayoutTemplate {...ICON_PROPS} size={14} className="shrink-0" />
        <span className="truncate">{choice.template.content.name}</span>
        <ChevronDown {...ICON_PROPS} size={14} className="shrink-0" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-h-[360px] w-64 overflow-y-auto">
        <DropdownMenuLabel className="font-normal text-pretty text-fg-muted">
          {reasonText(choice.reason)}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {choice.reason.kind === 'chosen' ? <BackToAuto onPick={onPick} /> : null}
        <TemplateGroups current={choice.template.id} onPick={onPick} />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The user's templates, then the built-in ones. */
function TemplateGroups({
  current,
  onPick,
}: {
  current: TemplateId;
  onPick: (id: TemplateId) => void;
}) {
  const all = useTemplates().data ?? [];
  return (
    <>
      <MenuGroup
        label={templatesCopy.yours}
        templates={all.filter((t) => !t.isBuiltin)}
        current={current}
        onPick={onPick}
      />
      <MenuGroup
        label={templatesCopy.builtIn}
        templates={all.filter((t) => t.isBuiltin)}
        current={current}
        onPick={onPick}
      />
    </>
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

/** Props of {@link TemplateSwitchedPrompt}. */
export interface TemplateSwitchedPromptProps {
  /** The template just chosen. */
  name: string;
  onRegenerate: () => void;
  onDismiss: () => void;
}

/** "Notes were written before you chose X." with Regenerate and Not now. */
export function TemplateSwitchedPrompt({
  name,
  onRegenerate,
  onDismiss,
}: TemplateSwitchedPromptProps) {
  return (
    <div
      role="status"
      className="mb-3 flex items-center gap-3 rounded-lg bg-accent-soft px-3 py-2 text-sm text-fg"
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
