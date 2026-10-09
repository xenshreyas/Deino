import type { DragEvent, KeyboardEvent } from 'react';

import { cn } from '@/ui/cn';
import { ArrowDown, ArrowUp, GripVertical, ICON_PROPS, Trash2 } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Input, Textarea } from '@/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { templatesCopy } from '../copy';
import type { DraftSection } from '../draft';
import type { SectionFormat } from '../types';

/** The formats, in menu order. */
const FORMATS: readonly SectionFormat[] = ['bullets', 'paragraph', 'table', 'todo'];

/** What a section card reports. */
export interface SectionCardActions {
  onChange: (patch: Partial<DraftSection>) => void;
  /** Moves the section by `offset` places (−1 up, +1 down). */
  onMove: (offset: number) => void;
  onRemove: () => void;
  onDragStart: () => void;
  onDragOver: () => void;
  onDrop: () => void;
}

/** Props of {@link SectionCard}. */
export interface SectionCardProps extends SectionCardActions {
  section: DraftSection;
  position: number;
  count: number;
  readOnly: boolean;
  /** True while this card is the one being dragged. */
  dragging: boolean;
  /** True while a dragged card hovers over this one. */
  dropTarget: boolean;
}

/** ↑/↓ move the section by one place; `withAlt` requires Alt (inside fields). */
function moveKeys(onMove: (offset: number) => void, withAlt: boolean) {
  return (event: KeyboardEvent) => {
    const offset = { ArrowUp: -1, ArrowDown: 1 }[event.key];
    if (offset === undefined || (withAlt && !event.altKey)) return;
    event.preventDefault();
    event.stopPropagation();
    onMove(offset);
  };
}

/** The list item's drag-and-drop and keyboard handlers. */
function itemHandlers(props: SectionCardProps) {
  if (props.readOnly) return {};
  return {
    draggable: true,
    onDragStart: props.onDragStart,
    onDragOver: (event: DragEvent) => {
      event.preventDefault();
      props.onDragOver();
    },
    onDrop: (event: DragEvent) => {
      event.preventDefault();
      props.onDrop();
    },
    onKeyDown: moveKeys(props.onMove, true),
  };
}

/** One section of the template: handle, title, format, instructions. */
export function SectionCard(props: SectionCardProps) {
  const { section, readOnly, dragging, dropTarget } = props;
  return (
    <li
      aria-label={section.title}
      {...itemHandlers(props)}
      className={cn(
        'group flex gap-2 px-3 py-3 transition-[opacity,background-color] duration-(--dur-fast)',
        dragging && 'bg-bg-muted opacity-60',
        dropTarget && 'shadow-[inset_0_2px_0_var(--accent)]',
      )}
    >
      <Handle {...props} />
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <SectionHeading {...props} />
        <Textarea
          aria-label={`${templatesCopy.instructions}: ${section.title}`}
          placeholder={templatesCopy.instructionsPlaceholder}
          value={section.instructions}
          readOnly={readOnly}
          rows={2}
          className="min-h-14 resize-y"
          onChange={(event) => {
            props.onChange({ instructions: event.target.value });
          }}
        />
      </div>
    </li>
  );
}

/** The drag handle, also a keyboard control. */
function Handle({ section, readOnly, onMove }: SectionCardProps) {
  if (readOnly) return <span className="w-6 shrink-0" />;
  return (
    <button
      type="button"
      aria-label={templatesCopy.dragSection(section.title)}
      title={templatesCopy.sectionsHint}
      onKeyDown={moveKeys(onMove, false)}
      className="mt-1 flex size-6 shrink-0 cursor-grab items-center justify-center rounded-sm text-fg-subtle hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring active:cursor-grabbing"
    >
      <GripVertical {...ICON_PROPS} />
    </button>
  );
}

/** Title, format and the move and remove buttons. */
function SectionHeading(props: SectionCardProps) {
  const { section, readOnly, position, count, onChange } = props;
  return (
    <div className="flex items-center gap-2">
      <Input
        aria-label={templatesCopy.sectionTitle}
        value={section.title}
        readOnly={readOnly}
        className="font-semibold"
        onChange={(event) => {
          onChange({ title: event.target.value });
        }}
      />
      <FormatSelect
        value={section.format}
        disabled={readOnly}
        onChange={(format) => {
          onChange({ format });
        }}
      />
      {readOnly ? null : (
        <SectionButtons {...props} first={position === 0} last={position === count - 1} />
      )}
    </div>
  );
}

/** Up, down and remove. */
function SectionButtons({
  onMove,
  onRemove,
  first,
  last,
  count,
}: SectionCardProps & { first: boolean; last: boolean }) {
  return (
    <div className="flex shrink-0 items-center">
      <IconButton
        icon={ArrowUp}
        label={templatesCopy.moveUp}
        disabled={first}
        onClick={() => {
          onMove(-1);
        }}
      />
      <IconButton
        icon={ArrowDown}
        label={templatesCopy.moveDown}
        disabled={last}
        onClick={() => {
          onMove(1);
        }}
      />
      <IconButton
        icon={Trash2}
        label={templatesCopy.removeSection}
        disabled={count === 1}
        onClick={onRemove}
      />
    </div>
  );
}

/** The section's format. */
function FormatSelect({
  value,
  disabled,
  onChange,
}: {
  value: SectionFormat;
  disabled: boolean;
  onChange: (format: SectionFormat) => void;
}) {
  return (
    <Select
      value={value}
      disabled={disabled}
      onValueChange={(next) => {
        const format = FORMATS.find((candidate) => candidate === next);
        if (format !== undefined) onChange(format);
      }}
    >
      <SelectTrigger aria-label={templatesCopy.format} className="w-32 shrink-0">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {FORMATS.map((format) => (
          <SelectItem key={format} value={format}>
            {templatesCopy.formats[format]}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
