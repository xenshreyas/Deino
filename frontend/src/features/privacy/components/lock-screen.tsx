/**
 * The lock screen (PRV-02): shown instead of the app while Deino is locked.
 * It reads nothing from the library; recording keeps running underneath,
 * and its controls stay here so a meeting can still be stopped.
 */
import { RecordingControls, useRecordingState } from '@/features/recording';
import { Button } from '@/ui/button';
import { ICON_PROPS, ICON_PROPS_LARGE, Lock } from '@/ui/icons';

import { privacyCopy } from '../copy';

const copy = privacyCopy.lock;

/** Props of {@link LockScreenView}. */
export interface LockScreenViewProps {
  /** True while the system prompt is open. */
  unlocking: boolean;
  /** True while a meeting is being recorded. */
  recording: boolean;
  onUnlock: () => void;
}

/** The lock screen's layout, without data. */
export function LockScreenView({ unlocking, recording, onUnlock }: LockScreenViewProps) {
  return (
    <div className="relative flex h-screen w-screen flex-col bg-bg text-fg">
      <div data-tauri-drag-region className="h-11 shrink-0" />
      <main className="flex flex-1 flex-col items-center justify-center gap-6 px-6 pb-11">
        <div className="flex size-12 items-center justify-center rounded-full bg-bg-muted text-fg-muted">
          <Lock {...ICON_PROPS_LARGE} />
        </div>
        <div className="flex max-w-sm flex-col items-center gap-1 text-center">
          <h1 className="text-2xl font-semibold">{copy.title}</h1>
          <p className="text-sm text-fg-muted">{copy.description}</p>
        </div>
        <Button size="lg" autoFocus disabled={unlocking} onClick={onUnlock}>
          <Lock {...ICON_PROPS} />
          {unlocking ? copy.unlocking : copy.unlock}
        </Button>
        {recording ? (
          <div className="flex flex-col items-center gap-2 pt-6">
            <RecordingControls />
            <p className="text-xs text-fg-muted">{copy.recording}</p>
          </div>
        ) : null}
      </main>
    </div>
  );
}

/** Props of {@link LockScreen}. */
export interface LockScreenProps {
  unlocking: boolean;
  onUnlock: () => void;
}

/** The lock screen over the live recorder state. */
export function LockScreen({ unlocking, onUnlock }: LockScreenProps) {
  const recorder = useRecordingState();
  return (
    <LockScreenView
      unlocking={unlocking}
      recording={recorder.phase !== 'idle'}
      onUnlock={onUnlock}
    />
  );
}
