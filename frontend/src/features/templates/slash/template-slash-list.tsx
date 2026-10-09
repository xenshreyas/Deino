import { cn } from '@/ui/cn';
import { ICON_PROPS, LayoutTemplate } from '@/ui/icons';

import { templatesCopy } from '../copy';
import type { Template } from '../types';

/** Props of {@link TemplateSlashList}. */
export interface TemplateSlashListProps {
  items: readonly Template[];
  /** The highlighted row, moved by the editor's ↑/↓ handling. */
  selected: number;
  onSelect: (template: Template) => void;
}

/**
 * The `/template` menu: a popover list of templates with their section
 * count, rendered by the notepad's suggestion renderer (NOT-01).
 */
export function TemplateSlashList({ items, selected, onSelect }: TemplateSlashListProps) {
  return (
    <div
      role="listbox"
      aria-label={templatesCopy.pickerLabel}
      className="flex w-72 flex-col gap-px rounded-lg bg-surface-raised p-1 shadow-popover"
    >
      {items.map((template, index) => (
        <button
          key={template.id}
          type="button"
          role="option"
          aria-selected={index === selected}
          onMouseDown={(event) => {
            event.preventDefault();
            onSelect(template);
          }}
          className={cn(
            'flex h-[30px] items-center gap-2 rounded-sm px-2 text-left text-sm text-fg',
            index === selected && 'bg-bg-muted',
          )}
        >
          <LayoutTemplate {...ICON_PROPS} className="shrink-0 text-fg-muted" />
          <span className="min-w-0 flex-1 truncate">{template.content.name}</span>
          <span className="shrink-0 text-xs text-fg-subtle tabular-nums">
            {templatesCopy.sectionCount(template.content.sections.length)}
          </span>
        </button>
      ))}
    </div>
  );
}
