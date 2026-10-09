/**
 * The app shell around every route: providers, the first-run onboarding gate,
 * the sidebar layout, audio drag-and-drop import and the tray listener.
 *
 * Ported from the Next.js root layout (`app/layout.tsx`). FND-10 replaces the
 * layout with the §4.6 shell and nav registry.
 */
import type { ReactNode } from 'react';
import { Toaster } from 'sonner';
import 'sonner/dist/styles.css';

import MainContent from '@/components/MainContent';
import { OnboardingFlow } from '@/components/onboarding';
import { DownloadProgressToastProvider } from '@/components/shared/DownloadProgressToast';
import Sidebar from '@/components/Sidebar';

import { AppProviders } from './app-providers';
import { AudioDropTarget } from './components/audio-drop-target';
import { ImportDialogGate } from './components/import-dialog-gate';
import { TrayRecordingToggle } from './components/tray-recording-toggle';
import { useImportDialogState } from './hooks/use-import-dialog-state';
import { useOnboardingGate } from './hooks/use-onboarding-gate';
import { useProductionContextMenuGuard } from './hooks/use-production-context-menu-guard';

interface MainLayoutProps {
  onAudioFile: (path: string) => void;
  children: ReactNode;
}

/** Sidebar plus the current route; accepts dropped audio files. */
function MainLayout({ onAudioFile, children }: MainLayoutProps) {
  return (
    <>
      <div className="flex">
        <Sidebar />
        <MainContent>{children}</MainContent>
      </div>
      <AudioDropTarget onAudioFile={onAudioFile} />
    </>
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
      <Toaster position="bottom-center" richColors closeButton />
    </>
  );
}
