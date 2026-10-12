/**
 * The Insights page (PARITY_PLAN AI-06; Circleback §10.3): the user's
 * insights on the left, the selected one's table of answers across
 * meetings on the right, "New insight" opening the template gallery and
 * then the editor.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { ICON_PROPS, Lightbulb, Plus } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';

import { insightsCopy } from '../copy';
import { type EditorTarget, InsightEditor } from '../editor/insight-editor';
import { useInsights } from '../queries';
import type { InsightId, InsightInput, InsightSummary } from '../types';
import { GalleryDialog } from './gallery-dialog';
import { InsightPanel } from './insight-panel';
import { InsightRail } from './insight-rail';

/** Props of {@link InsightsScreen}; the route keeps the selection in the URL. */
export interface InsightsScreenProps {
  selected: InsightId | undefined;
  onSelect: (id: InsightId | undefined) => void;
}

/** The gallery and the editor, and how one leads to the other. */
type Creation = ReturnType<typeof useCreation>;

/** The state behind {@link Creation}. */
function useCreation(onSelect: (id: InsightId) => void) {
  const [gallery, setGallery] = useState(false);
  const [editor, setEditor] = useState<EditorTarget | null>(null);
  return {
    gallery,
    editor,
    openGallery: () => {
      setGallery(true);
    },
    setGallery,
    choose: (input: InsightInput) => {
      setGallery(false);
      setEditor({ id: null, input });
    },
    edit: (target: EditorTarget) => {
      setEditor(target);
    },
    close: () => {
      setEditor(null);
    },
    saved: (id: InsightId) => {
      setEditor(null);
      onSelect(id);
    },
  };
}

/** The page. */
export function InsightsScreen({ selected, onSelect }: InsightsScreenProps) {
  const insights = useInsights();
  const creation = useCreation(onSelect);
  const list = insights.data ?? [];
  const current = list.find((summary) => summary.def.id === selected) ?? list[0];
  const newButton = (
    <Button onClick={creation.openGallery}>
      <Plus {...ICON_PROPS} />
      {insightsCopy.newInsight}
    </Button>
  );
  return (
    <Page className="max-w-[1200px]">
      <PageHeader
        title={insightsCopy.title}
        description={insightsCopy.description}
        actions={newButton}
      />
      {insights.isPending ? (
        <RailSkeleton />
      ) : (
        <Body list={list} current={current} onSelect={onSelect} creation={creation} />
      )}
      <Dialogs creation={creation} />
    </Page>
  );
}

/** The gallery and the editor. */
function Dialogs({ creation }: { creation: Creation }) {
  return (
    <>
      <GalleryDialog
        open={creation.gallery}
        onOpenChange={creation.setGallery}
        onChoose={creation.choose}
      />
      <InsightEditor
        target={creation.editor}
        onClose={creation.close}
        onSaved={(def) => {
          creation.saved(def.id);
        }}
      />
    </>
  );
}

/** The rail and the selected insight, or the empty state. */
function Body({
  list,
  current,
  onSelect,
  creation,
}: {
  list: readonly InsightSummary[];
  current: InsightSummary | undefined;
  onSelect: (id: InsightId | undefined) => void;
  creation: Creation;
}) {
  if (current === undefined) return <NoInsights onCreate={creation.openGallery} />;
  return (
    <div className="grid grid-cols-1 gap-8 md:grid-cols-[13rem_minmax(0,1fr)]">
      <InsightRail insights={list} selected={current.def.id} onSelect={onSelect} />
      <InsightPanel
        key={current.def.id}
        id={current.def.id}
        onEdit={(def) => {
          creation.edit({ id: def.id, input: def });
        }}
        onDeleted={() => {
          onSelect(undefined);
        }}
      />
    </div>
  );
}

/** The first visit: one action, the gallery. */
function NoInsights({ onCreate }: { onCreate: () => void }) {
  return (
    <EmptyState
      icon={Lightbulb}
      title={insightsCopy.empty.title}
      description={insightsCopy.empty.body}
      className="pt-[12vh]"
      action={
        <Button variant="secondary" onClick={onCreate}>
          {insightsCopy.empty.action}
        </Button>
      }
    />
  );
}

function RailSkeleton() {
  return (
    <div aria-hidden className="grid grid-cols-[13rem_1fr] gap-8">
      <div className="grid gap-2">
        {[0, 1, 2].map((index) => (
          <Skeleton key={index} className="h-4 w-36" />
        ))}
      </div>
      <Skeleton className="h-5 w-48" />
    </div>
  );
}
