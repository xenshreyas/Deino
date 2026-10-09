/**
 * The `/template` menu: NOT-04's `TemplateSlashList`, placed under the typed
 * command and driven by the editor's keys. It never takes focus.
 */
import { createPortal } from 'react-dom';
import { useStore } from 'zustand';

import { type Template, TemplateSlashList } from '@/features/templates';

import type { MenuStore } from '../editor/menu-store';

/** Gap between the typed command and the menu, in px. */
const MENU_OFFSET = 6;

/** The menu for one editor; nothing while no `/template` is typed or nothing matches. */
export function TemplateMenu({ store }: { store: MenuStore<Template> }) {
  const session = useStore(store, (state) => state.session);
  const active = useStore(store, (state) => state.active);
  if (session?.rect == null || session.items.length === 0) return null;
  const { rect, items, choose } = session;
  return createPortal(
    <div
      className="fixed z-(--z-popover)"
      style={{ top: rect.bottom + MENU_OFFSET, left: rect.left }}
    >
      <TemplateSlashList items={items} selected={active} onSelect={choose} />
    </div>,
    document.body,
  );
}
