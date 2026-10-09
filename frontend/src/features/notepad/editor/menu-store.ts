/**
 * The state of a suggestion menu (@mentions, /template), shared by TipTap's
 * suggestion plugin, which drives it, and the React menu, which draws it.
 * One store per menu per editor.
 */
import { createStore } from 'zustand/vanilla';

import type { MentionCandidate } from '../types';

/** What the suggestion plugin reports while a trigger is being typed. */
export interface MenuSession<T> {
  items: T[];
  /** Where the typed query is on screen, for placing the menu. */
  rect: DOMRect | null;
  /** Applies the chosen item and ends the session. */
  choose: (item: T) => void;
}

/** A menu's state and transitions. */
export interface MenuState<T> {
  session: MenuSession<T> | null;
  active: number;
  /** Starts or refreshes the session; the highlight stays in range. */
  show: (session: MenuSession<T>) => void;
  hide: () => void;
  /** Handles a key while the menu is open; true when it was consumed. */
  key: (key: string) => boolean;
}

/** Keys that move the highlight, by how far. */
const MOVES: Readonly<Record<string, number>> = { ArrowDown: 1, ArrowUp: -1 };

/** A store for one menu. */
export function createMenuStore<T>() {
  return createStore<MenuState<T>>()((set, get) => ({
    session: null,
    active: 0,
    show: (session) => {
      set((state) => ({
        session,
        active: Math.min(state.active, Math.max(session.items.length - 1, 0)),
      }));
    },
    hide: () => {
      set({ session: null, active: 0 });
    },
    key: (key) => {
      const { session, active } = get();
      if (session === null || session.items.length === 0) return false;
      const move = MOVES[key];
      if (move !== undefined) {
        const count = session.items.length;
        set({ active: (active + move + count) % count });
        return true;
      }
      const chosen = session.items[active];
      if ((key === 'Enter' || key === 'Tab') && chosen !== undefined) {
        session.choose(chosen);
        return true;
      }
      if (key === 'Escape') get().hide();
      return key === 'Escape';
    },
  }));
}

/** One menu's store. */
export type MenuStore<T> = ReturnType<typeof createMenuStore<T>>;

/** The @mention menu's store. */
export type MentionStore = MenuStore<MentionCandidate>;

/** A store for the @mention menu. */
export function createMentionStore(): MentionStore {
  return createMenuStore<MentionCandidate>();
}

/** What a suggestion plugin's renderer receives, as far as a menu needs. */
interface RenderProps<T> {
  items: T[];
  clientRect?: (() => DOMRect | null) | null;
}

/**
 * A suggestion plugin's `render()` result that drives `store`; `choose`
 * applies an item through the plugin's own `command`.
 */
export function suggestionRenderer<T, P extends RenderProps<T>>(
  store: MenuStore<T> | null,
  choose: (props: P, item: T) => void,
) {
  const show = (props: P) => {
    store?.getState().show({
      items: props.items,
      rect: props.clientRect?.() ?? null,
      choose: (item) => {
        choose(props, item);
      },
    });
  };
  return {
    onStart: show,
    onUpdate: show,
    onExit: () => store?.getState().hide(),
    onKeyDown: ({ event }: { event: KeyboardEvent }) => store?.getState().key(event.key) ?? false,
  };
}
