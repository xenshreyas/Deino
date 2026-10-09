/**
 * Skeleton: a pulsing `bg-muted` block in the shape of the content that is
 * loading (ui.md §3 Skeleton). Loading never shows a spinner in content.
 */
import { cn } from './cn';

/** Props of {@link Skeleton}. */
export interface SkeletonProps {
  /** Size and shape, for example `h-2.5 w-3/4`. */
  className?: string;
}

/** One placeholder block; reduced motion stops the pulse (globals.css). */
export function Skeleton({ className }: SkeletonProps) {
  return <div aria-hidden className={cn('animate-pulse-soft rounded-xs bg-bg-muted', className)} />;
}

/** Line widths that read as real text (ui.md: 100/92/76/60%). */
const LINE_WIDTHS = ['w-full', 'w-[92%]', 'w-[76%]', 'w-[60%]'] as const;

/** Props of {@link SkeletonText}. */
export interface SkeletonTextProps {
  /** How many lines; widths cycle through 100/92/76/60%. */
  lines?: number;
  className?: string;
}

/** A paragraph of skeleton lines sized for 14px text. */
export function SkeletonText({ lines = 4, className }: SkeletonTextProps) {
  return (
    <div role="status" aria-label="Loading" className={cn('flex flex-col gap-2.5', className)}>
      {Array.from({ length: lines }, (_, index) => (
        <Skeleton key={index} className={cn('h-2.5', LINE_WIDTHS[index % LINE_WIDTHS.length])} />
      ))}
    </div>
  );
}
