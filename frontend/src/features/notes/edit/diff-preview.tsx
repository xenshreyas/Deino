/**
 * The proposed edit: how many changes (or that a rename needed no AI), the
 * Inline / Side by side switch, the changed blocks, and Reject / Accept.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Check, ICON_PROPS, LoaderCircle } from '@/ui/icons';
import { SegmentedControl } from '@/ui/segmented-control';

import { editCopy } from './copy';
import { DiffRows } from './diff-rows';
import type { DiffLayout, EditProposal } from './types';

const LAYOUTS = [
  { value: 'inline', label: editCopy.inline },
  { value: 'split', label: editCopy.split },
] as const;

/** Props of {@link DiffPreview}. */
export interface DiffPreviewProps {
  proposal: EditProposal;
  accepting: boolean;
  onAccept: () => void;
  onReject: () => void;
}

/** The proposal, for the user to accept or reject. */
export function DiffPreview({ proposal, accepting, onAccept, onReject }: DiffPreviewProps) {
  const [layout, setLayout] = useState<DiffLayout>('inline');
  return (
    <div className="flex flex-col gap-3 border-t border-border px-3 pt-3 pb-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-fg">
          <span className="font-medium">{editCopy.previewTitle}</span>
          <span className="text-fg-muted"> · {summary(proposal)}</span>
        </p>
        <SegmentedControl
          label={editCopy.layoutLabel}
          segments={LAYOUTS}
          value={layout}
          onValueChange={setLayout}
        />
      </div>
      <div className="max-h-[40vh] overflow-y-auto pr-1">
        <DiffRows rows={proposal.changes} layout={layout} />
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" onClick={onReject} disabled={accepting}>
          {editCopy.reject}
        </Button>
        <Button size="sm" onClick={onAccept} disabled={accepting} aria-busy={accepting}>
          {accepting ? (
            <LoaderCircle {...ICON_PROPS} className="animate-spin" />
          ) : (
            <Check {...ICON_PROPS} />
          )}
          {editCopy.accept}
        </Button>
      </div>
    </div>
  );
}

/** "3 changes", or "Renamed in 4 places, without AI". */
function summary(proposal: EditProposal): string {
  if (proposal.method === 'rename' && proposal.occurrences !== null) {
    return editCopy.renamed(proposal.occurrences);
  }
  return editCopy.changes(proposal.changes.length);
}
