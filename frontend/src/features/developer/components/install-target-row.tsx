import { useState } from 'react';

import { Button } from '@/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';

import { cliCopy } from '../copy';
import type { InstallTarget } from '../types';

/** Props of {@link InstallTargetRow}. */
export interface InstallTargetRowProps {
  target: InstallTarget;
  /** True while any install runs. */
  busy: boolean;
  /** Installs into this target's folder, after the user confirmed. */
  onInstall: () => void;
}

/**
 * One install folder: where the link goes, what is there now, and an
 * Install button that asks before it changes anything.
 */
export function InstallTargetRow({ target, busy, onInstall }: InstallTargetRowProps) {
  const [confirming, setConfirming] = useState(false);
  const dir = cliCopy.dirs[target.dir];
  const installed = target.state.kind === 'installed';
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="font-mono text-sm text-fg">{dir.label}</p>
        <p className="mt-0.5 text-xs text-fg-muted">
          {stateLabel(target)} · {target.writable ? dir.hint : cliCopy.needsAdmin}
        </p>
      </div>
      <Button
        variant="secondary"
        size="sm"
        disabled={busy || target.state.kind === 'other_file'}
        onClick={() => {
          setConfirming(true);
        }}
      >
        {installed ? cliCopy.reinstall : cliCopy.install}
      </Button>
      <ConfirmInstall
        open={confirming}
        path={target.path}
        onOpenChange={setConfirming}
        onConfirm={() => {
          setConfirming(false);
          onInstall();
        }}
      />
    </div>
  );
}

/** What is at the target now, in words. */
function stateLabel(target: InstallTarget): string {
  return cliCopy.states[target.state.kind];
}

/** Props of {@link ConfirmInstall}. */
interface ConfirmInstallProps {
  open: boolean;
  path: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}

/** Names the exact link the install creates before it does. */
function ConfirmInstall({ open, path, onOpenChange, onConfirm }: ConfirmInstallProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{cliCopy.confirmTitle}</DialogTitle>
          <DialogDescription>{cliCopy.confirmBody(path)}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="ghost">{cliCopy.cancel}</Button>
          </DialogClose>
          <Button onClick={onConfirm}>{cliCopy.install}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
