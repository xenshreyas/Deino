/**
 * The selected insight's header on the Insights page: its name, when it
 * runs and its prompt; "Run on past meetings", "Export CSV" and a menu
 * with Edit and Delete; then the run's progress while meetings are queued.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Progress } from '@/ui/controls';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { Download, Ellipsis, ICON_PROPS, Pencil, Play, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { insightsCopy } from '../copy';
import { runState } from '../model';
import { useDeleteInsight, useExportInsightCsv } from '../queries';
import type { InsightDef, RunProgress } from '../types';

const copy = insightsCopy.actions;

/** Props of {@link InsightHeader}. */
export interface InsightHeaderProps {
  def: InsightDef;
  progress: RunProgress;
  onRun: () => void;
  onEdit: () => void;
  onDeleted: () => void;
}

/** When the insight runs, in a few words. */
export function applyLabel(def: InsightDef): string {
  if (def.applyWhen.mode === 'manual') return insightsCopy.apply.manual;
  return def.applyWhen.conditions.length === 0
    ? insightsCopy.apply.always
    : insightsCopy.apply.some;
}

/** Name, rules, prompt and actions. */
export function InsightHeader({ def, progress, onRun, onEdit, onDeleted }: InsightHeaderProps) {
  const [confirming, setConfirming] = useState(false);
  const fields = insightsCopy.gallery.fields(def.fields.length);
  return (
    <header className="pb-5">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 className="text-lg font-semibold text-balance text-fg">{def.name}</h2>
          <p className="mt-0.5 text-xs text-fg-muted">
            {applyLabel(def)} · {fields}
          </p>
        </div>
        <Actions
          def={def}
          onRun={onRun}
          onEdit={onEdit}
          onDelete={() => {
            setConfirming(true);
          }}
        />
      </div>
      <p
        className="mt-3 line-clamp-2 max-w-[65ch] text-sm text-pretty text-fg-muted"
        title={def.description}
      >
        {def.description}
      </p>
      <RunLine progress={progress} />
      <DeleteDialog
        def={def}
        open={confirming}
        onOpenChange={setConfirming}
        onDeleted={onDeleted}
      />
    </header>
  );
}

/** Run on past meetings, Export CSV, and the menu. */
function Actions({
  def,
  onRun,
  onEdit,
  onDelete,
}: {
  def: InsightDef;
  onRun: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="flex shrink-0 items-center gap-2">
      <Button variant="secondary" size="sm" onClick={onRun}>
        <Play {...ICON_PROPS} />
        {copy.runPast}
      </Button>
      <ExportButton def={def} />
      <MoreMenu onEdit={onEdit} onDelete={onDelete} />
    </div>
  );
}

/** "Export CSV": the save dialog opens in Rust; a toast names the file. */
function ExportButton({ def }: { def: InsightDef }) {
  const exportCsv = useExportInsightCsv();
  const run = () => {
    exportCsv.mutate(def.id, {
      onSuccess: (saved) => {
        if (saved !== null) toast.success(copy.exported(saved.fileName));
      },
      onError: (error) => toast.error(error.message),
    });
  };
  return (
    <Button variant="secondary" size="sm" disabled={exportCsv.isPending} onClick={run}>
      <Download {...ICON_PROPS} />
      {copy.exportCsv}
    </Button>
  );
}

/** Edit and Delete. */
function MoreMenu({ onEdit, onDelete }: { onEdit: () => void; onDelete: () => void }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={copy.more}>
          <Ellipsis {...ICON_PROPS} />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={onEdit}>
          <Pencil {...ICON_PROPS} />
          {copy.edit}
        </DropdownMenuItem>
        <DropdownMenuItem destructive onSelect={onDelete}>
          <Trash2 {...ICON_PROPS} />
          {copy.delete}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** "Running on 3 meetings · 2 of 5 done" with a bar, while any are queued. */
function RunLine({ progress }: { progress: RunProgress }) {
  const { total, waiting } = runState(progress);
  if (waiting === 0 && progress.failed === 0) return null;
  const done = total - waiting;
  return (
    <div className="mt-4 grid max-w-sm gap-1.5" role="status">
      {waiting > 0 ? (
        <>
          <p className="text-xs text-fg-muted tabular-nums">
            {insightsCopy.progress.running(waiting, total)}
          </p>
          <Progress value={total === 0 ? 0 : (done / total) * 100} aria-label={copy.runPast} />
        </>
      ) : null}
      {progress.failed > 0 ? (
        <p className="text-xs text-danger">{insightsCopy.progress.failed(progress.failed)}</p>
      ) : null}
    </div>
  );
}

/** Deletes `def` and closes the dialog; a failure becomes a toast. */
function useConfirmDelete(def: InsightDef, onDeleted: () => void, close: () => void) {
  const remove = useDeleteInsight();
  return () => {
    remove.mutate(def.id, {
      onSuccess: onDeleted,
      onError: (error) => toast.error(error.message),
    });
    close();
  };
}

/** Confirms deleting the insight and its answers. */
function DeleteDialog({
  def,
  open,
  onOpenChange,
  onDeleted,
}: {
  def: InsightDef;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDeleted: () => void;
}) {
  const close = () => {
    onOpenChange(false);
  };
  const confirm = useConfirmDelete(def, onDeleted, close);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent hideClose>
        <DialogHeader>
          <DialogTitle>{copy.deleteTitle(def.name)}</DialogTitle>
          <DialogDescription>{copy.deleteBody}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={close}>
            {copy.cancel}
          </Button>
          {/* The dialog exists to confirm this one action. */}
          <Button variant="danger-solid" autoFocus onClick={confirm}>
            {copy.confirmDelete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
