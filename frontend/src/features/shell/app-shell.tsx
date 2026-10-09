/**
 * The app shell around every route (PARITY_PLAN §4.6): providers, the
 * first-run onboarding gate, the sidebar and title bar, the ⌘K palette, the
 * ⌘J chat bar, global shortcuts, audio drag-and-drop import, and the tray and
 * meeting-detection recording listeners.
 */
import type { ReactNode } from 'react';

import { OnboardingFlow } from '@/components/onboarding';
import { DownloadProgressToastProvider } from '@/components/shared/DownloadProgressToast';
import { Toaster } from '@/ui/toaster';

import { AppProviders } from './app-providers';
import { CommandPalette } from './command-palette/command-palette';
import { DetectionRecordingStart } from './components/detection-recording-start';
import { AudioDropTarget } from './components/audio-drop-target';
import { ChatPlaceholder } from './components/chat-placeholder';
import { ImportDialogGate } from './components/import-dialog-gate';
import { TitleBar } from './components/title-bar';
import { TrayRecordingToggle } from './components/tray-recording-toggle';
import { useFocusMode } from './hooks/use-focus-mode';
import { useGlobalShortcuts } from './hooks/use-global-shortcuts';
import { useImportDialogState } from './hooks/use-import-dialog-state';
import { useOnboardingGate } from './hooks/use-onboarding-gate';
import { useProductionContextMenuGuard } from './hooks/use-production-context-menu-guard';
import { Sidebar } from './sidebar/sidebar';

interface MainLayoutProps {
  onAudioFile: (path: string) => void;
  children: ReactNode;
}

/** Sidebar, title bar and the current route, plus the global overlays. */
function MainLayout({ onAudioFile, children }: MainLayoutProps) {
  useGlobalShortcuts();
  const focusMode = useFocusMode();
  return (
    <div
      data-focus-mode={focusMode}
      className="flex h-screen w-screen overflow-hidden bg-bg text-fg"
    >
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TitleBar />
        <main className="relative min-h-0 flex-1">{children}</main>
      </div>
      <CommandPalette />
      <ChatPlaceholder />
      <AudioDropTarget onAudioFile={onAudioFile} />
    </div>
  );
}

/** Renders `children` (the matched route) inside the app chrome. */
export function AppShell({ children }: { children: ReactNode }) {
  const { showOnboarding, completeOnboarding } = useOnboardingGate();
  const importDialog = useImportDialogState();
  useProductionContextMenuGuard();

  return (
    <>
      <AppProviders onOpenImportDialog={importDialog.openWith}>
        <DownloadProgressToastProvider />
        <TrayRecordingToggle key={String(showOnboarding)} onboarding={showOnboarding} />
        <DetectionRecordingStart
          key={`detection-${String(showOnboarding)}`}
          onboarding={showOnboarding}
        />
        {showOnboarding ? (
          <OnboardingFlow onComplete={completeOnboarding} />
        ) : (
          <MainLayout onAudioFile={importDialog.openWith}>{children}</MainLayout>
        )}
        <ImportDialogGate
          open={importDialog.open}
          filePath={importDialog.filePath}
          onOpenChange={importDialog.onOpenChange}
        />
      </AppProviders>
      <Toaster />
    </>
  );
}
