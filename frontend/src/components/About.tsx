import React, { useEffect, useState } from 'react';
import { ModelLicenceNotice } from '@/features/transcription';
import { getVersion } from '@tauri-apps/api/app';
import { CheckCircle2, Loader2 } from 'lucide-react';
import { toast } from 'sonner';
import { UpdateDialog } from './UpdateDialog';
import { updateService, UpdateInfo } from '@/services/updateService';
import { Button } from '@/ui/button';

/** Short statements about where data lives, shown under the version. */
const PRIVACY_POINTS: readonly string[] = [
  'Recording, transcription and storage run on this Mac.',
  'Audio and transcripts never leave your computer.',
  'Notes are sent only to the AI provider you choose.',
  'No telemetry, tracking or accounts.',
];

/**
 * The About panel: app identity, version, privacy summary and update status.
 *
 * Update checks are gated by the Rust-side update gate; while it is off the
 * panel says so instead of offering a check button.
 */
export function About() {
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  const [updatesEnabled, setUpdatesEnabled] = useState<boolean | null>(null);

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(console.error);
    updateService.isEnabled().then(setUpdatesEnabled).catch(console.error);
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center gap-4">
        <img src="/icon_128x128.png" alt="" width={48} height={48} />
        <div className="flex flex-col gap-0.5">
          <h3 className="text-sm font-semibold text-fg">Deino</h3>
          {currentVersion && <p className="text-xs text-fg-muted">Version {currentVersion}</p>}
          <p className="text-xs text-fg-muted">A private, local-first AI meeting notepad.</p>
        </div>
      </div>

      <ul className="space-y-1.5 text-sm text-fg-muted">
        {PRIVACY_POINTS.map((point) => (
          <li key={point} className="flex items-start gap-2">
            <CheckCircle2 className="h-4 w-4 mt-0.5 shrink-0 text-fg-subtle" />
            <span>{point}</span>
          </li>
        ))}
      </ul>

      <ModelLicenceNotice />

      <div className="pt-4 border-t border-border">
        {updatesEnabled === false && (
          <p className="text-xs text-fg-muted">
            Automatic updates are off in this build. Install new versions manually.
          </p>
        )}
        {updatesEnabled === true && <UpdateCheck />}
      </div>
    </div>
  );
}

/** "Check for Updates" button plus the install dialog; only shown when updates are enabled. */
function UpdateCheck() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [showDialog, setShowDialog] = useState(false);

  const handleCheck = async () => {
    setIsChecking(true);
    try {
      const info = await updateService.checkForUpdates(true);
      setUpdateInfo(info);
      if (info.available) {
        setShowDialog(true);
      } else {
        toast.success('You are running the latest version');
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      toast.error(`Failed to check for updates: ${message}`);
    } finally {
      setIsChecking(false);
    }
  };

  return (
    <>
      <Button onClick={handleCheck} disabled={isChecking} variant="secondary" size="sm" className="text-xs">
        {isChecking ? (
          <Loader2 className="h-3 w-3 mr-2 animate-spin" />
        ) : (
          <CheckCircle2 className="h-3 w-3 mr-2" />
        )}
        {isChecking ? 'Checking…' : 'Check for updates'}
      </Button>
      {updateInfo?.available && (
        <p className="mt-2 text-xs text-accent">Update available: v{updateInfo.version}</p>
      )}
      <UpdateDialog open={showDialog} onOpenChange={setShowDialog} updateInfo={updateInfo} />
    </>
  );
}
