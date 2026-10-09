/**
 * Every design token, drawn: colours, type scale, radii, shadows and motion
 * (docs/references/ui.md §2).
 */
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';

/** The colour tokens, with the utility class that paints each one. */
const COLOURS = [
  { name: 'bg', swatch: 'bg-bg' },
  { name: 'bg-subtle', swatch: 'bg-bg-subtle' },
  { name: 'bg-muted', swatch: 'bg-bg-muted' },
  { name: 'surface', swatch: 'bg-surface' },
  { name: 'surface-raised', swatch: 'bg-surface-raised' },
  { name: 'fg', swatch: 'bg-fg' },
  { name: 'fg-muted', swatch: 'bg-fg-muted' },
  { name: 'fg-subtle', swatch: 'bg-fg-subtle' },
  { name: 'ai-text', swatch: 'bg-ai-text' },
  { name: 'border', swatch: 'bg-border' },
  { name: 'border-strong', swatch: 'bg-border-strong' },
  { name: 'accent', swatch: 'bg-accent' },
  { name: 'accent-soft', swatch: 'bg-accent-soft' },
  { name: 'selection', swatch: 'bg-selection' },
  { name: 'highlight', swatch: 'bg-highlight' },
  { name: 'recording', swatch: 'bg-recording' },
  { name: 'danger', swatch: 'bg-danger' },
  { name: 'danger-soft', swatch: 'bg-danger-soft' },
  { name: 'warning', swatch: 'bg-warning' },
  { name: 'warning-soft', swatch: 'bg-warning-soft' },
  { name: 'success', swatch: 'bg-success' },
  { name: 'speaker-1', swatch: 'bg-speaker-1' },
  { name: 'speaker-2', swatch: 'bg-speaker-2' },
  { name: 'speaker-3', swatch: 'bg-speaker-3' },
  { name: 'speaker-4', swatch: 'bg-speaker-4' },
  { name: 'speaker-5', swatch: 'bg-speaker-5' },
  { name: 'speaker-6', swatch: 'bg-speaker-6' },
] as const;

const TYPE_SCALE = [
  { name: 'text-xl · 24/32 · 600', className: 'text-xl font-semibold' },
  { name: 'text-base · 16/26', className: 'text-base' },
  { name: 'text-sm · 14/20', className: 'text-sm' },
  { name: 'text-xs · 12/16', className: 'text-xs' },
] as const;

const RADII = [
  'rounded-xs',
  'rounded-sm',
  'rounded-md',
  'rounded-lg',
  'rounded-xl',
  'rounded-full',
] as const;
const SHADOWS = [
  'shadow-xs',
  'shadow-popover',
  'shadow-dialog',
  'shadow-chatbar',
  'shadow-pill',
] as const;

/** A labelled block of the gallery. */
export function GallerySection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-xs font-semibold text-fg-muted">{title}</h3>
      {children}
    </section>
  );
}

/** Colour swatches with their token names. */
function ColourSwatches() {
  return (
    <div className="grid grid-cols-3 gap-2">
      {COLOURS.map((colour) => (
        <div key={colour.name} className="flex items-center gap-2">
          <span className={cn('size-6 shrink-0 rounded-sm border border-border', colour.swatch)} />
          <span className="truncate text-xs text-fg-muted">{colour.name}</span>
        </div>
      ))}
    </div>
  );
}

/** The type scale and the AI / user text colours. */
function TypeScale() {
  return (
    <GallerySection title="Type">
      {TYPE_SCALE.map((step) => (
        <p key={step.name} className={cn('text-fg', step.className)}>
          {step.name}
        </p>
      ))}
      <p className="text-sm text-ai-text">AI-written text (ai-text)</p>
      <p className="text-sm text-user-text">Your own words (user-text)</p>
    </GallerySection>
  );
}

/** Radius and shadow scales. */
function ShapeScales() {
  return (
    <>
      <GallerySection title="Radius">
        <div className="flex gap-2">
          {RADII.map((radius) => (
            <div
              key={radius}
              title={radius}
              className={cn('size-10 border border-border-strong bg-bg-muted', radius)}
            />
          ))}
        </div>
      </GallerySection>
      <GallerySection title="Shadow (floating layers only)">
        <div className="flex flex-wrap gap-4">
          {SHADOWS.map((shadow) => (
            <div
              key={shadow}
              className={cn(
                'flex h-12 w-24 items-center justify-center rounded-lg bg-surface-raised text-xs text-fg-muted',
                shadow,
              )}
            >
              {shadow.replace('shadow-', '')}
            </div>
          ))}
        </div>
      </GallerySection>
    </>
  );
}

/** Every token. */
export function TokenGallery() {
  return (
    <div className="flex flex-col gap-6">
      <GallerySection title="Colour">
        <ColourSwatches />
      </GallerySection>
      <TypeScale />
      <ShapeScales />
    </div>
  );
}
