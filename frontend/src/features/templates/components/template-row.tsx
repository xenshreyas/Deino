import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';
import { Copy, Download, Ellipsis, ICON_PROPS, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { templatesCopy } from '../copy';
import type { Template } from '../types';

/** What a row's menu does. */
export interface TemplateRowActions {
  onDuplicate: (template: Template) => void;
  onExport: (template: Template) => void;
  onDelete: (template: Template) => void;
}

/** Props of {@link TemplateRow}. */
export interface TemplateRowProps extends TemplateRowActions {
  template: Template;
}

/** A two-line row: name, then its description; ⋯ on hover. */
export function TemplateRow({ template, ...actions }: TemplateRowProps) {
  const { name, description, sections, autoRules } = template.content;
  const shape = `${templatesCopy.sectionCount(sections.length)}${templatesCopy.ruleCount(autoRules.length)}`;
  return (
    <li className="group relative">
      <Link
        to="/settings"
        search={{ section: 'templates', template: template.id }}
        className="flex min-h-14 flex-col justify-center rounded-md px-3 py-2 pr-10 transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <span className="truncate text-sm text-fg">{name}</span>
        <span className="truncate text-xs text-fg-muted">
          {description === '' ? shape : description}
        </span>
      </Link>
      <RowMenu template={template} {...actions} />
    </li>
  );
}

/** Duplicate, Copy as JSON and (for the user's own) Delete. */
function RowMenu({ template, onDuplicate, onExport, onDelete }: TemplateRowProps) {
  return (
    <DropdownMenu>
      <RowTrigger name={template.content.name} />
      <DropdownMenuContent align="end">
        <MenuAction
          icon={<Copy {...ICON_PROPS} />}
          label={templatesCopy.duplicate}
          onSelect={() => {
            onDuplicate(template);
          }}
        />
        <MenuAction
          icon={<Download {...ICON_PROPS} />}
          label={templatesCopy.export}
          onSelect={() => {
            onExport(template);
          }}
        />
        {template.isBuiltin ? null : (
          <>
            <DropdownMenuSeparator />
            <MenuAction
              destructive
              icon={<Trash2 {...ICON_PROPS} />}
              label={templatesCopy.delete}
              onSelect={() => {
                onDelete(template);
              }}
            />
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The ⋯ button, shown on hover and focus. */
function RowTrigger({ name }: { name: string }) {
  return (
    <DropdownMenuTrigger
      aria-label={templatesCopy.actionsFor(name)}
      className={cn(
        'absolute top-1/2 right-1.5 flex size-7 -translate-y-1/2 items-center justify-center rounded-sm text-fg-muted',
        'opacity-0 transition-opacity duration-(--dur-fast) group-hover:opacity-100 hover:bg-bg focus-visible:opacity-100 data-[state=open]:opacity-100',
      )}
    >
      <Ellipsis {...ICON_PROPS} />
    </DropdownMenuTrigger>
  );
}

function MenuAction(props: {
  icon: ReactNode;
  label: string;
  destructive?: boolean;
  onSelect: () => void;
}) {
  return (
    <DropdownMenuItem destructive={props.destructive ?? false} onSelect={props.onSelect}>
      {props.icon}
      {props.label}
    </DropdownMenuItem>
  );
}
