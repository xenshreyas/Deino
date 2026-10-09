import { ImportAudioDialog } from '@/components/ImportAudio';
import { useConfig } from '@/contexts/ConfigContext';

import type { ImportDialogState } from '../hooks/use-import-dialog-state';

type ImportDialogGateProps = Pick<ImportDialogState, 'open' | 'filePath' | 'onOpenChange'>;

/**
 * Mounts the audio import dialog only while its beta feature is on, so its
 * hooks and listeners don't run otherwise. Must sit inside `ConfigProvider`.
 */
export function ImportDialogGate({ open, filePath, onOpenChange }: ImportDialogGateProps) {
  const { betaFeatures } = useConfig();
  if (!betaFeatures.importAndRetranscribe) return null;

  return <ImportAudioDialog open={open} onOpenChange={onOpenChange} preselectedFile={filePath} />;
}
