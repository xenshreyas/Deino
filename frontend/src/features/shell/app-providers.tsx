import type { ReactNode } from 'react';

import { SidebarProvider } from '@/components/Sidebar/SidebarProvider';
import { TooltipProvider } from '@/components/ui/tooltip';
import { UpdateCheckProvider } from '@/components/UpdateCheckProvider';
import { ConfigProvider } from '@/contexts/ConfigContext';
import { ImportDialogProvider } from '@/contexts/ImportDialogContext';
import { OllamaDownloadProvider } from '@/contexts/OllamaDownloadContext';
import { OnboardingProvider } from '@/contexts/OnboardingContext';
import { RecordingPostProcessingProvider } from '@/contexts/RecordingPostProcessingProvider';
import { RecordingStateProvider } from '@/contexts/RecordingStateContext';
import { TranscriptProvider } from '@/contexts/TranscriptContext';

interface AppProvidersProps {
  /** Opens the audio import dialog; exposed to the tree via `useImportDialog`. */
  onOpenImportDialog: (filePath?: string | null) => void;
  children: ReactNode;
}

/**
 * The app-wide React contexts, in dependency order (outermost first).
 *
 * The order is load-bearing: later providers read earlier ones (for example
 * `SidebarProvider` reads recording state). Rust data is moving out of these
 * contexts into TanStack Query (the meetings list already lives in
 * `@/features/meetings`); the query client itself is provided in `main.tsx`.
 */
export function AppProviders({ onOpenImportDialog, children }: AppProvidersProps) {
  return (
    <RecordingStateProvider>
      <TranscriptProvider>
        <ConfigProvider>
          <OllamaDownloadProvider>
            <OnboardingProvider>
              <UpdateCheckProvider>
                <SidebarProvider>
                  <TooltipProvider>
                    <RecordingPostProcessingProvider>
                      <ImportDialogProvider onOpen={onOpenImportDialog}>
                        {children}
                      </ImportDialogProvider>
                    </RecordingPostProcessingProvider>
                  </TooltipProvider>
                </SidebarProvider>
              </UpdateCheckProvider>
            </OnboardingProvider>
          </OllamaDownloadProvider>
        </ConfigProvider>
      </TranscriptProvider>
    </RecordingStateProvider>
  );
}
