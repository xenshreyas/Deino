/**
 * The ⌘K command palette (ui.md §3 Command palette): a 640px dialog 18vh from
 * the top with one search field and the registered providers' groups.
 */
import * as DialogPrimitive from '@radix-ui/react-dialog';
import { useState } from 'react';

import { type FilterChip, FilterChips } from '@/features/search';
import { Command, CommandEmpty, CommandInput, CommandList } from '@/ui/command';
import { VisuallyHidden } from '@/ui/controls';

import { useShellLayout } from '../layout-store';
import { paletteCopy } from './copy';
import { PALETTE_PROVIDERS } from './providers';

const PROVIDERS = [...PALETTE_PROVIDERS].sort((a, b) => a.order - b.order);

/** Keyboard hints in the palette footer. */
function PaletteFooter() {
  return (
    <div className="flex h-8 items-center gap-3 border-t border-border px-4 text-xs text-fg-subtle">
      <span>↵ {paletteCopy.hintOpen}</span>
      <span>↑↓ {paletteCopy.hintNavigate}</span>
      <span>esc {paletteCopy.hintClose}</span>
    </div>
  );
}

/** The palette body; mounted only while open, so search and filters start empty. */
function PaletteBody({ close }: { close: () => void }) {
  const [search, setSearch] = useState('');
  const [filters, setFilters] = useState<FilterChip[]>([]);
  const removeFilter = (removed: FilterChip) => {
    setFilters((current) => current.filter((chip) => chip !== removed));
  };
  const props = { search, close, filters, onFiltersChange: setFilters, onSearchChange: setSearch };
  return (
    <Command
      loop
      label={paletteCopy.title}
      onKeyDown={(event) => {
        // Backspace in an empty input takes off the last filter.
        if (event.key === 'Backspace' && search === '' && filters.length > 0) {
          setFilters((current) => current.slice(0, -1));
        }
      }}
    >
      <CommandInput
        variant="palette"
        value={search}
        onValueChange={setSearch}
        placeholder={filters.length === 0 ? paletteCopy.placeholder : ''}
        autoFocus
        before={<FilterChips filters={filters} onRemove={removeFilter} className="shrink-0" />}
      />
      <CommandList>
        <CommandEmpty>{paletteCopy.empty}</CommandEmpty>
        {PROVIDERS.map(({ id, Component }) => (
          <Component key={id} {...props} />
        ))}
      </CommandList>
      <PaletteFooter />
    </Command>
  );
}

/** The palette dialog, opened by ⌘K, the sidebar search row or the store. */
export function CommandPalette() {
  const open = useShellLayout((state) => state.paletteOpen);
  const setOpen = useShellLayout((state) => state.setPaletteOpen);
  const close = () => {
    setOpen(false);
  };
  return (
    <DialogPrimitive.Root open={open} onOpenChange={setOpen}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-(--z-overlay) bg-overlay/50 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
        <DialogPrimitive.Content
          aria-describedby={undefined}
          className="fixed top-[18vh] left-1/2 z-(--z-palette) w-[calc(100%-2rem)] max-w-[640px] -translate-x-1/2 overflow-hidden rounded-lg bg-surface-raised shadow-dialog duration-(--dur-slow) ease-out data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98]"
        >
          <VisuallyHidden>
            <DialogPrimitive.Title>{paletteCopy.title}</DialogPrimitive.Title>
          </VisuallyHidden>
          <PaletteBody close={close} />
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
