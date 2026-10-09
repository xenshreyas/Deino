/**
 * Tag pill (ui.md §3 Tag pill): 22px, hairline border, 12px muted label and
 * a 6px dot in the tag's colour. Dense rows use the borderless {@link TagMark}.
 */
import { cn } from '@/ui/cn';
import { ICON_PROPS, Sparkles, X } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { tagsCopy } from '../copy';
import { paletteSlot } from '../palette';
import type { PaletteColor, Tag } from '../types';

/** The 6px colour dot. */
export function TagDot({ color, className }: { color: PaletteColor; className?: string }) {
  return (
    <span
      aria-hidden
      className={cn('size-1.5 shrink-0 rounded-full', paletteSlot(color).dot, className)}
    />
  );
}

/** Dot plus label with no border, for list rows. */
export function TagMark({ tag }: { tag: Tag }) {
  return (
    <span className="inline-flex min-w-0 items-center gap-1.5 text-xs text-fg-muted">
      <TagDot color={tag.color} />
      <span className="truncate">{tag.name}</span>
    </span>
  );
}

/** Props of {@link TagPill}. */
export interface TagPillProps {
  tag: Tag;
  /** True when the auto-tagger applied it; shows a quiet sparkle. */
  byDeino?: boolean;
  /** Shows a × on hover that takes the tag off. */
  onRemove?: (() => void) | undefined;
}

/** A tag on a meeting, removable when `onRemove` is given. */
export function TagPill({ tag, byDeino = false, onRemove }: TagPillProps) {
  return (
    <span className="group/pill inline-flex h-[22px] items-center gap-1.5 rounded-full border border-border px-2 text-xs text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted">
      <TagDot color={tag.color} />
      <span className="max-w-40 truncate">{tag.name}</span>
      {byDeino ? (
        <Hint label={tagsCopy.addedByDeino}>
          <span
            tabIndex={0}
            role="img"
            aria-label={tagsCopy.addedByDeino}
            className="flex rounded-sm text-fg-subtle"
          >
            <Sparkles {...ICON_PROPS} size={12} />
          </span>
        </Hint>
      ) : null}
      {onRemove === undefined ? null : (
        <button
          type="button"
          aria-label={tagsCopy.remove(tag.name)}
          onClick={onRemove}
          className="-ml-1.5 flex h-4 w-0 items-center justify-center overflow-hidden rounded-full text-fg-subtle opacity-0 transition-[width,opacity,margin] duration-(--dur-fast) ease-out group-hover/pill:ml-0 group-hover/pill:w-4 group-hover/pill:opacity-100 hover:text-fg focus-visible:ml-0 focus-visible:w-4 focus-visible:opacity-100"
        >
          <X {...ICON_PROPS} size={12} />
        </button>
      )}
    </span>
  );
}
