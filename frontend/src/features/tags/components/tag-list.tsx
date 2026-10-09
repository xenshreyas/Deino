/**
 * The Tags page's list: one two-line row per tag (ui.md §3 List row): dot
 * and name over the description, the meeting count and a ⋯ menu trailing.
 */
import { cn } from '@/ui/cn';
import { Ellipsis, ICON_PROPS, Pencil, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { tagsCopy } from '../copy';
import type { Tag } from '../types';
import { TagDot } from './tag-pill';

/** Props of {@link TagList}. */
export interface TagListProps {
  tags: readonly Tag[];
  /** Meetings per tag ID; absent means none. */
  counts: ReadonlyMap<string, number>;
  onEdit: (tag: Tag) => void;
  onDelete: (tag: Tag) => void;
}

function RowMenu({
  tag,
  onEdit,
  onDelete,
}: { tag: Tag } & Pick<TagListProps, 'onEdit' | 'onDelete'>) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={tagsCopy.rowActions(tag.name)}
        className={cn(
          'flex size-7 shrink-0 items-center justify-center rounded-sm text-fg-muted',
          'opacity-0 transition-opacity duration-(--dur-fast) group-hover:opacity-100 hover:bg-bg focus-visible:opacity-100 data-[state=open]:opacity-100',
        )}
      >
        <Ellipsis {...ICON_PROPS} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          onSelect={() => {
            onEdit(tag);
          }}
        >
          <Pencil {...ICON_PROPS} />
          {tagsCopy.edit}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          destructive
          onSelect={() => {
            onDelete(tag);
          }}
        >
          <Trash2 {...ICON_PROPS} />
          {tagsCopy.delete}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function TagRow({
  tag,
  count,
  onEdit,
  onDelete,
}: { tag: Tag; count: number } & Pick<TagListProps, 'onEdit' | 'onDelete'>) {
  return (
    <li className="group flex h-14 items-center gap-3 rounded-md pr-1.5 transition-colors duration-(--dur-fast) hover:bg-bg-muted has-[button[data-row]:focus-visible]:bg-bg-muted">
      <button
        type="button"
        data-row
        onClick={() => {
          onEdit(tag);
        }}
        className="flex h-full min-w-0 flex-1 items-center gap-3 rounded-md pl-3 text-left focus-visible:outline-offset-[-2px]"
      >
        <TagDot color={tag.color} className="size-2" />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm text-fg">{tag.name}</span>
          <span
            className={cn(
              'truncate text-xs',
              tag.description === '' ? 'text-fg-subtle' : 'text-fg-muted',
            )}
          >
            {tag.description === '' ? tagsCopy.noDescription : tag.description}
          </span>
        </span>
        <span className="shrink-0 text-xs text-fg-muted tabular-nums">
          {tagsCopy.meetingCount(count)}
        </span>
      </button>
      <RowMenu tag={tag} onEdit={onEdit} onDelete={onDelete} />
    </li>
  );
}

/** Every tag, by name. */
export function TagList({ tags, counts, onEdit, onDelete }: TagListProps) {
  return (
    <ul className="flex flex-col">
      {tags.map((tag) => (
        <TagRow
          key={tag.id}
          tag={tag}
          count={counts.get(tag.id) ?? 0}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      ))}
    </ul>
  );
}
