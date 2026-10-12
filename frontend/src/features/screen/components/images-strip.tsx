/**
 * The meeting page's Images strip (CAP-09): the snapshots kept of the
 * shared screen, in time order, as a row of thumbnails above the notes.
 * Clicking one opens it large with its time and a delete button. Nothing
 * shows for a meeting without snapshots.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Dialog, DialogContent, DialogTitle } from '@/ui/dialog';
import { Trash2 } from '@/ui/icons';

import { snapshotSrc } from '../api';
import { screenCopy } from '../copy';
import { useDeleteSnapshot, useSnapshotFolder, useSnapshots } from '../queries';
import type { MeetingId, Snapshot } from '../types';

const copy = screenCopy.images;

/** `12:04`, or `1:12:04` from an hour on. */
export function snapshotTime(at: number): string {
  const seconds = Math.floor(at / 1000);
  const [hours, minutes, rest] = [
    Math.floor(seconds / 3600),
    Math.floor(seconds / 60) % 60,
    seconds % 60,
  ];
  const mmss = `${String(minutes).padStart(2, '0')}:${String(rest).padStart(2, '0')}`;
  return hours > 0 ? `${String(hours)}:${mmss}` : mmss;
}

/** Props of {@link ImagesStripView}. */
export interface ImagesStripViewProps {
  snapshots: readonly Snapshot[];
  /** The picture's URL. */
  src: (snapshot: Snapshot) => string;
  onDelete: (snapshot: Snapshot) => void;
  deleting: boolean;
}

/** The strip and its enlarged picture. Presentational. */
export function ImagesStripView({ snapshots, src, onDelete, deleting }: ImagesStripViewProps) {
  const [open, setOpen] = useState<Snapshot | null>(null);
  if (snapshots.length === 0) return null;
  return (
    <section aria-label={copy.title} className="flex flex-col gap-2">
      <h3 className="text-xs font-medium text-fg-muted">{copy.title}</h3>
      <ul className="flex gap-2 overflow-x-auto pb-1">
        {snapshots.map((snapshot) => (
          <li key={snapshot.id} className="shrink-0">
            <button
              type="button"
              aria-label={copy.open(snapshotTime(snapshot.at))}
              onClick={() => {
                setOpen(snapshot);
              }}
              className="block overflow-hidden rounded-md border border-border transition-opacity duration-(--dur-fast) hover:opacity-85 focus-visible:ring-2 focus-visible:ring-accent focus-visible:outline-none"
            >
              <img src={src(snapshot)} alt="" loading="lazy" className="h-16 w-28 object-cover" />
            </button>
          </li>
        ))}
      </ul>
      <Enlarged
        snapshot={open}
        src={src}
        deleting={deleting}
        onClose={() => {
          setOpen(null);
        }}
        onDelete={(snapshot) => {
          onDelete(snapshot);
          setOpen(null);
        }}
      />
    </section>
  );
}

/** What {@link Enlarged} shows and does. */
interface EnlargedProps {
  snapshot: Snapshot | null;
  src: (snapshot: Snapshot) => string;
  deleting: boolean;
  onClose: () => void;
  onDelete: (snapshot: Snapshot) => void;
}

/** One snapshot, large, with its time and a delete button. */
function EnlargedBody({
  snapshot,
  src,
  deleting,
  onDelete,
}: EnlargedProps & { snapshot: Snapshot }) {
  return (
    <>
      <DialogTitle className="text-sm font-medium">
        {copy.open(snapshotTime(snapshot.at))}
      </DialogTitle>
      <img src={src(snapshot)} alt="" className="w-full rounded-md border border-border" />
      <div className="flex justify-end">
        <Button
          variant="ghost"
          size="sm"
          disabled={deleting}
          onClick={() => {
            onDelete(snapshot);
          }}
        >
          <Trash2 size={14} strokeWidth={1.5} aria-hidden />
          {copy.delete}
        </Button>
      </div>
    </>
  );
}

/** The dialog around the enlarged snapshot. */
function Enlarged(props: EnlargedProps) {
  const { snapshot, onClose } = props;
  return (
    <Dialog
      open={snapshot !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent className="max-w-[min(960px,calc(100%-2rem))] gap-3 p-4">
        {snapshot === null ? null : <EnlargedBody {...props} snapshot={snapshot} />}
      </DialogContent>
    </Dialog>
  );
}

/** The strip of `meetingId`. */
export function MeetingImages({ meetingId }: { meetingId: MeetingId }) {
  const snapshots = useSnapshots(meetingId);
  const folder = useSnapshotFolder(meetingId);
  const remove = useDeleteSnapshot();
  if (snapshots.data === undefined || folder.data === undefined) return null;
  const root = folder.data;
  return (
    <ImagesStripView
      snapshots={snapshots.data}
      src={(snapshot) => snapshotSrc(root, snapshot.fileName)}
      deleting={remove.isPending}
      onDelete={(snapshot) => {
        remove.mutate(
          { meetingId, id: snapshot.id },
          {
            onError: (error) => {
              console.error('[screen] Could not delete the snapshot', error);
            },
          },
        );
      }}
    />
  );
}
