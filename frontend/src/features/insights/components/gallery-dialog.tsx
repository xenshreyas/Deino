/**
 * The template gallery: twelve ready-made insights in a quiet two-column
 * list, and "Start from scratch". Choosing one opens the editor with it, so
 * the user can adjust it before anything is created.
 */
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/ui/dialog';
import { ICON_PROPS, Plus } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { insightsCopy } from '../copy';
import { BLANK_INPUT } from '../editor/draft';
import { useInsightTemplates } from '../queries';
import type { InsightInput } from '../types';

/** Props of {@link GalleryDialog}. */
export interface GalleryDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** A template (or a blank insight) was chosen. */
  onChoose: (input: InsightInput) => void;
}

const copy = insightsCopy.gallery;

/** The gallery, in a dialog. */
export function GalleryDialog({ open, onOpenChange, onChoose }: GalleryDialogProps) {
  const templates = useInsightTemplates();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[720px]">
        <DialogHeader>
          <DialogTitle>{copy.title}</DialogTitle>
          <DialogDescription>{copy.description}</DialogDescription>
        </DialogHeader>
        <ul aria-label={copy.title} className="-mx-2 grid grid-cols-1 gap-x-2 sm:grid-cols-2">
          <li>
            <Choice
              name={copy.scratch}
              summary={copy.scratchSummary}
              meta={null}
              onClick={() => {
                onChoose(BLANK_INPUT);
              }}
              leading
            />
          </li>
          {templates.data?.map((template) => (
            <li key={template.key}>
              <Choice
                name={template.input.name}
                summary={template.summary}
                meta={copy.fields(template.input.fields.length)}
                onClick={() => {
                  onChoose(template.input);
                }}
              />
            </li>
          )) ?? <Loading />}
        </ul>
      </DialogContent>
    </Dialog>
  );
}

/** One template: its name, one line, how many fields. */
function Choice({
  name,
  summary,
  meta,
  onClick,
  leading = false,
}: {
  name: string;
  summary: string;
  meta: string | null;
  onClick: () => void;
  leading?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full items-start gap-3 rounded-md px-2 py-2.5 text-left transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted focus-visible:outline-2 focus-visible:outline-focus-ring"
    >
      {leading ? (
        <span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-sm border border-border text-fg-muted">
          <Plus {...ICON_PROPS} size={12} />
        </span>
      ) : null}
      <span className="min-w-0">
        <span className="flex items-baseline gap-2">
          <span className="text-sm font-semibold text-fg">{name}</span>
          {meta === null ? null : <span className="text-xs text-fg-subtle">{meta}</span>}
        </span>
        <span className="mt-0.5 block text-sm text-pretty text-fg-muted">{summary}</span>
      </span>
    </button>
  );
}

function Loading() {
  return (
    <>
      {[0, 1, 2, 3].map((index) => (
        <li key={index} className="px-2 py-2.5">
          <Skeleton className="h-3.5 w-32" />
          <Skeleton className="mt-2 h-3 w-56" />
        </li>
      ))}
    </>
  );
}
