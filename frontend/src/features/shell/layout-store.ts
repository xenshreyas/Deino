/**
 * Shared shell UI state: sidebar and right panel visibility, sidebar width,
 * and the palette and chat overlays. A small Zustand store, because the
 * sidebar, the keyboard shortcuts, the palette and the meeting page all read
 * it (architecture.md (b) FND-10). Nothing here is persisted yet; FND-06's
 * settings store will keep the sidebar width and collapsed state.
 */
import { create } from 'zustand';

/** Sidebar width bounds and default, in px (ui.md §3 Sidebar). */
export const SIDEBAR_WIDTH = { min: 200, default: 232, max: 320 } as const;

/** Clamps a requested sidebar width into its bounds. */
export function clampSidebarWidth(width: number): number {
  return Math.round(Math.min(SIDEBAR_WIDTH.max, Math.max(SIDEBAR_WIDTH.min, width)));
}

/** The shell's layout state and its transitions. */
export interface ShellLayout {
  sidebarOpen: boolean;
  sidebarWidth: number;
  rightPanelOpen: boolean;
  paletteOpen: boolean;
  chatOpen: boolean;
  /** Focus mode: the chrome fades while the user types in a note; any pointer movement ends it. */
  focusMode: boolean;
  /** Where pages portal their title-bar actions (see `TitleBarActions`). */
  titleBarSlot: HTMLElement | null;
  toggleSidebar: () => void;
  setSidebarWidth: (width: number) => void;
  toggleRightPanel: () => void;
  setPaletteOpen: (open: boolean) => void;
  setChatOpen: (open: boolean) => void;
  setTitleBarSlot: (slot: HTMLElement | null) => void;
  setFocusMode: (focusMode: boolean) => void;
}

/** Builds a layout store; tests create their own. */
export function createShellLayoutStore() {
  return create<ShellLayout>()((set) => ({
    sidebarOpen: true,
    sidebarWidth: SIDEBAR_WIDTH.default,
    rightPanelOpen: false,
    paletteOpen: false,
    chatOpen: false,
    titleBarSlot: null,
    focusMode: false,
    toggleSidebar: () => {
      set((state) => ({ sidebarOpen: !state.sidebarOpen }));
    },
    setSidebarWidth: (width) => {
      set({ sidebarWidth: clampSidebarWidth(width) });
    },
    toggleRightPanel: () => {
      set((state) => ({ rightPanelOpen: !state.rightPanelOpen }));
    },
    setPaletteOpen: (paletteOpen) => {
      set({ paletteOpen, chatOpen: false });
    },
    setChatOpen: (chatOpen) => {
      set({ chatOpen, paletteOpen: false });
    },
    setTitleBarSlot: (titleBarSlot) => {
      set({ titleBarSlot });
    },
    setFocusMode: (focusMode) => {
      set((state) => (state.focusMode === focusMode ? state : { focusMode }));
    },
  }));
}

/** The app's shell layout. Select single fields: `useShellLayout((s) => s.sidebarOpen)`. */
export const useShellLayout = createShellLayoutStore();
