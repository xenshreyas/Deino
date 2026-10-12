/**
 * The avatar of a person or company (ui.md §3 Speaker chip): initials in a
 * soft tone of the speaker palette, picked from the ID so someone keeps
 * their colour everywhere. People are round, companies rounded squares; a
 * cached company logo replaces the initials, and a person at a company
 * with a logo wears it as a small badge.
 */
import { useState } from 'react';

import { speakerTone } from '@/features/speakers';
import { cn } from '@/ui/cn';

import { logoUrl } from '../api';
import { initials, paletteSlot } from '../format';

/** The sizes the pages use: rows 20/24, hover cards 32, page headers 48. */
export type AvatarSize = 20 | 24 | 32 | 48;

const SIZES: Record<AvatarSize, string> = {
  20: 'size-5 text-[9px]',
  24: 'size-6 text-[10px]',
  32: 'size-8 text-xs',
  48: 'size-12 text-base',
};

/** Props of {@link EntityAvatar}. */
export interface EntityAvatarProps {
  id: string;
  name: string;
  kind: 'person' | 'company';
  /** A cached logo: the company's own, or a person's company's badge. */
  logoPath?: string | null | undefined;
  size?: AvatarSize;
  className?: string;
}

/** A logo image that falls back to nothing when the file cannot be read. */
function Logo({ path, className }: { path: string; className: string }) {
  const [broken, setBroken] = useState(false);
  if (broken) return null;
  return (
    <img
      src={logoUrl(path)}
      alt=""
      draggable={false}
      onError={() => {
        setBroken(true);
      }}
      className={className}
    />
  );
}

/** Initials, a logo, or initials wearing a company badge. */
export function EntityAvatar({
  id,
  name,
  kind,
  logoPath = null,
  size = 24,
  className,
}: EntityAvatarProps) {
  const tone = speakerTone(paletteSlot(id));
  const shape = kind === 'person' ? 'rounded-full' : 'rounded-md';
  const companyLogo = kind === 'company' && logoPath !== null;
  return (
    <span
      aria-hidden
      className={cn(
        'relative inline-flex shrink-0 items-center justify-center font-semibold select-none',
        SIZES[size],
        shape,
        companyLogo ? 'bg-surface ring-1 ring-border' : cn(tone.soft, tone.text),
        className,
      )}
    >
      {initials(name)}
      {logoPath === null ? null : (
        <Logo
          path={logoPath}
          className={
            companyLogo
              ? cn('absolute inset-0 size-full bg-surface object-contain p-[12%]', shape)
              : 'absolute -right-0.5 -bottom-0.5 size-[45%] rounded-[3px] bg-surface object-contain ring-1 ring-bg'
          }
        />
      )}
    </span>
  );
}
