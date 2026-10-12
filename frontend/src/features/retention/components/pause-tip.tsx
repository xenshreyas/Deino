/**
 * "Pause for sensitive discussions" (PRV-01): a small card under the record
 * pill (CAP-10), shown during one recording only. It is marked seen
 * (`privacy.pauseTipSeen`) when the user dismisses it or that recording
 * ends, so it never comes back.
 */
import { useQuery } from '@tanstack/react-query';

import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { ICON_PROPS, Pause } from '@/ui/icons';

import { retentionApi } from '../api';
import { retentionCopy } from '../copy';

const copy = retentionCopy.tip;

/** Props of {@link PauseTip}. */
export interface PauseTipProps {
  onDismiss: () => void;
}

/** The card. Presentational. */
export function PauseTip({ onDismiss }: PauseTipProps) {
  return (
    <section
      aria-label={copy.title}
      className="flex w-[300px] animate-in items-start gap-2.5 rounded-xl bg-surface-raised/95 px-3.5 py-3 shadow-pill backdrop-blur-xl duration-(--dur-slow) ease-out fade-in-0 slide-in-from-top-1"
    >
      <Pause {...ICON_PROPS} className="mt-0.5 shrink-0 text-fg-muted" />
      <div className="flex min-w-0 flex-1 flex-col">
        <p className="text-xs font-semibold text-fg">{copy.title}</p>
        <p className="text-xs text-fg-muted">{copy.body}</p>
      </div>
      <Button variant="ghost" size="sm" className="h-7 px-2.5 text-xs" onClick={onDismiss}>
        {copy.dismiss}
      </Button>
    </section>
  );
}

/** Records that the tip was seen; it is a hint, so a failure is only logged. */
function markSeen(): void {
  retentionApi.pauseTipSeen().catch((error: unknown) => {
    console.error('[retention] Could not save that the pause tip was seen', error);
  });
}

/** The tip while it is up: seen once this recording ends. */
function ShownTip({ onDismiss }: PauseTipProps) {
  useMountEffect(() => markSeen);
  return (
    <PauseTip
      onDismiss={() => {
        markSeen();
        onDismiss();
      }}
    />
  );
}

/** The tip, until it has been seen once. Mount it only while recording. */
export function PauseTipSlot() {
  // The panel reads only this flag through `retention|pause_tip`, never the
  // settings document (its capability grants just the two tip commands).
  const tip = useQuery({ queryKey: ['retention', 'pause-tip'], queryFn: retentionApi.pauseTip });
  if (tip.data !== true) return null;
  return <ShownTip onDismiss={() => void tip.refetch()} />;
}
