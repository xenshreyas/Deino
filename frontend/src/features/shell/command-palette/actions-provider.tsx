import { useRouter } from '@tanstack/react-router';

import { useConfig } from '@/contexts/ConfigContext';
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { appearanceCopy, type ThemePreference } from '@/features/appearance';
import { useSaveTheme } from '@/features/settings';
import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import {
  type Icon,
  ICON_PROPS,
  ListChecks,
  MessageCircle,
  Mic,
  Monitor,
  Moon,
  PanelLeft,
  PanelRight,
  SquarePen,
  Sun,
  Upload,
} from '@/ui/icons';

import { useShellLayout } from '../layout-store';
import { requestRecordingStart } from '../recording';
import { SHORTCUTS } from '../shortcuts';
import { paletteCopy } from './copy';
import type { PaletteProviderProps } from './registry';

/** The palette's built-in actions. */
type ActionId =
  'new-note' | 'new-action-item' | 'record' | 'chat' | 'sidebar' | 'right-panel' | 'import';

/** How an action looks in the list. */
interface ActionView {
  id: ActionId;
  label: string;
  icon: Icon;
  shortcut?: string;
}

const ACTIONS: readonly ActionView[] = [
  {
    id: 'new-note',
    label: paletteCopy.newNote,
    icon: SquarePen,
    shortcut: SHORTCUTS.newNote.display,
  },
  { id: 'new-action-item', label: paletteCopy.newActionItem, icon: ListChecks },
  {
    id: 'record',
    label: paletteCopy.startRecording,
    icon: Mic,
    shortcut: SHORTCUTS.record.display,
  },
  {
    id: 'chat',
    label: paletteCopy.askDeino,
    icon: MessageCircle,
    shortcut: SHORTCUTS.chat.display,
  },
  {
    id: 'sidebar',
    label: paletteCopy.toggleSidebar,
    icon: PanelLeft,
    shortcut: SHORTCUTS.toggleSidebar.display,
  },
  {
    id: 'right-panel',
    label: paletteCopy.toggleRightPanel,
    icon: PanelRight,
    shortcut: SHORTCUTS.toggleRightPanel.display,
  },
  { id: 'import', label: paletteCopy.importAudio, icon: Upload },
];

const THEMES: readonly { value: ThemePreference; label: string; icon: Icon }[] = [
  { value: 'system', label: appearanceCopy.system, icon: Monitor },
  { value: 'light', label: appearanceCopy.light, icon: Sun },
  { value: 'dark', label: appearanceCopy.dark, icon: Moon },
];

/** What each action does, bound to the current router, layout and dialogs. */
function useActionRunners(): Record<ActionId, () => void> {
  const router = useRouter();
  const layout = useShellLayout();
  const { openImportDialog } = useImportDialog();
  const goHome = () => void router.navigate({ to: '/' });
  return {
    'new-note': goHome,
    'new-action-item': () => void router.navigate({ to: '/action-items', search: { add: true } }),
    record: () => {
      void requestRecordingStart(goHome);
    },
    chat: () => {
      layout.setChatOpen(true);
    },
    sidebar: layout.toggleSidebar,
    'right-panel': layout.toggleRightPanel,
    import: () => {
      openImportDialog();
    },
  };
}

/** One row of the group. */
function ActionItem({
  view,
  onRun,
}: {
  view: ActionView | (typeof THEMES)[number];
  onRun: () => void;
}) {
  const ActionIcon = view.icon;
  const shortcut = 'shortcut' in view ? view.shortcut : undefined;
  return (
    <CommandItem value={`action ${view.label}`} keywords={[view.label]} onSelect={onRun}>
      <ActionIcon {...ICON_PROPS} />
      {view.label}
      {shortcut === undefined ? null : <CommandShortcut>{shortcut}</CommandShortcut>}
    </CommandItem>
  );
}

/** "Actions": new note, recording, chat, panels, import (beta) and theme. */
export function ActionsProvider({ close }: PaletteProviderProps) {
  const runners = useActionRunners();
  const saveTheme = useSaveTheme();
  const { betaFeatures } = useConfig();
  const visible = ACTIONS.filter(
    (action) => action.id !== 'import' || betaFeatures.importAndRetranscribe,
  );
  return (
    <CommandGroup heading={paletteCopy.actions}>
      {visible.map((view) => (
        <ActionItem
          key={view.id}
          view={view}
          onRun={() => {
            close();
            runners[view.id]();
          }}
        />
      ))}
      {THEMES.map((theme) => (
        <ActionItem
          key={theme.value}
          view={{ ...theme, label: paletteCopy.theme(theme.label) }}
          onRun={() => {
            close();
            saveTheme(theme.value);
          }}
        />
      ))}
    </CommandGroup>
  );
}
