/**
 * One insight on the Insights page: its header and its table of answers
 * across meetings, or a calm empty state that points at "Run on past
 * meetings".
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Lightbulb, ICON_PROPS, Play } from '@/ui/icons';
import { Skeleton, SkeletonText } from '@/ui/skeleton';

import { insightsCopy } from '../copy';
import { columnsOf, rowsOf } from '../model';
import { useInsightTable } from '../queries';
import type { InsightDef, InsightId, InsightTable as Table } from '../types';
import { InsightHeader } from './insight-header';
import { InsightTable } from './insight-table';
import { RunDialog } from './run-dialog';

/** Props of {@link InsightPanel}. */
export interface InsightPanelProps {
  id: InsightId;
  onEdit: (def: InsightDef) => void;
  onDeleted: () => void;
}

/** The insight's header, table and run dialog. */
export function InsightPanel({ id, onEdit, onDeleted }: InsightPanelProps) {
  const table = useInsightTable(id);
  const [running, setRunning] = useState(false);
  if (table.data === undefined) return <PanelSkeleton />;
  const { def, progress } = table.data;
  return (
    <section aria-label={def.name} className="min-w-0">
      <InsightHeader
        def={def}
        progress={progress}
        onRun={() => {
          setRunning(true);
        }}
        onEdit={() => {
          onEdit(def);
        }}
        onDeleted={onDeleted}
      />
      <Answers
        table={table.data}
        onRun={() => {
          setRunning(true);
        }}
      />
      <RunDialog def={def} open={running} onOpenChange={setRunning} />
    </section>
  );
}

/** The table, or the empty state before any answer. */
function Answers({ table, onRun }: { table: Table; onRun: () => void }) {
  const rows = rowsOf(table);
  if (rows.length === 0) {
    return (
      <div className="flex flex-col items-start gap-3 border-t border-border pt-8">
        <div className="flex size-10 items-center justify-center rounded-lg bg-bg-muted text-fg-muted">
          <Lightbulb {...ICON_PROPS} size={20} />
        </div>
        <div>
          <h3 className="text-sm font-semibold text-fg">{insightsCopy.table.empty}</h3>
          <p className="mt-1 max-w-[46ch] text-sm text-pretty text-fg-muted">
            {insightsCopy.table.emptyBody}
          </p>
        </div>
        <Button variant="secondary" size="sm" onClick={onRun}>
          <Play {...ICON_PROPS} />
          {insightsCopy.actions.runPast}
        </Button>
      </div>
    );
  }
  return <InsightTable name={table.def.name} columns={columnsOf(table.def)} rows={rows} />;
}

function PanelSkeleton() {
  return (
    <div aria-hidden className="grid gap-4">
      <Skeleton className="h-5 w-48" />
      <Skeleton className="h-3 w-32" />
      <SkeletonText lines={6} className="mt-4" />
    </div>
  );
}
