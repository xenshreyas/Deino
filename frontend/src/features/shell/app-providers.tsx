import type { ReactNode } from 'react';

import { SidebarProvider } from '@/components/Sidebar/SidebarProvider';
import { TooltipProvider } from '@/ui/tooltip';
import { UpdateCheckProvider } from '@/components/UpdateCheckProvider';
import { ConfigProvider } from '@/contexts/ConfigContext';
import { ImportDialogProvider } from '@/contexts/ImportDialogContext';
import { OllamaDownloadProvider } from '@/contexts/OllamaDownloadContext';
import { OnboardingProvider } from '@/contexts/OnboardingContext';
import { RecordingSync } from '@/features/recording';
import { SettingsSync } from '@/features/settings';

interface AppProvidersProps {
  /** Opens the audio import dialog; exposed to the tree via `useImportDialog`. */
  onOpenImportDialog: (filePath?: string | null) => void;
  children: ReactNode;
}

/**
 * The app-wide React contexts, in dependency order (outermost first).
 *
 * The order is load-bearing: later providers read earlier ones. Rust data
 * is moving out of these contexts into TanStack Query (the meetings list
 * lives in `@/features/meetings`, the recorder in `@/features/recording`);
 * the query client itself is provided in `main.tsx`. `SettingsSync` keeps
 * the cached settings equal to the saved ones; `RecordingSync` keeps the
 * recorder's state current.
 */
export function AppProviders({ onOpenImportDialog, children }: AppProvidersProps) {
  return (
    <>
      <SettingsSync />
      <RecordingSync />
      <ConfigProvider>
        <OllamaDownloadProvider>
          <OnboardingProvider>
            <UpdateCheckProvider>
              <SidebarProvider>
                <TooltipProvider>
                  <ImportDialogProvider onOpen={onOpenImportDialog}>
                    {children}
                  </ImportDialogProvider>
                </TooltipProvider>
              </SidebarProvider>
            </UpdateCheckProvider>
          </OnboardingProvider>
        </OllamaDownloadProvider>
      </ConfigProvider>
    </>
  );
}
