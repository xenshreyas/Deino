/**
 * The Tags page (ORG-06): the auto-tagging switch, then every tag with its
 * description and meeting count; create, edit (colour, description,
 * auto-apply) and delete.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { EmptyState } from '@/ui/empty-state';
import { CircleAlert, ICON_PROPS, Plus, Tag as TagIcon } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';

import { tagsCopy } from '../copy';
import { useDeleteTag, useTagLinks, useTags } from '../queries';
import type { Tag } from '../types';
import { AutoTagSetting } from './auto-tag-setting';
import { TagDialog, type TagDialogTarget } from './tag-dialog';
import { TagList } from './tag-list';

/** Meetings per tag, from the tag links. */
function useTagCounts(): ReadonlyMap<string, number> {
  const links = useTagLinks();
  const counts = new Map<string, number>();
  for (const link of links.data ?? []) counts.set(link.tagId, (counts.get(link.tagId) ?? 0) + 1);
  return counts;
}

function ListSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1 pt-2">
      {Array.from({ length: 4 }, (_, index) => (
        <div key={index} className="flex h-14 items-center gap-3 px-3">
          <Skeleton className="size-2 rounded-full" />
          <div className="flex flex-1 flex-col gap-2">
            <Skeleton className="h-2.5 w-1/4" />
            <Skeleton className="h-2.5 w-2/3" />
          </div>
        </div>
      ))}
    </div>
  );
}

/** Confirms and deletes a tag. */
function DeleteTagDialog({ tag, onClose }: { tag: Tag | null; onClose: () => void }) {
  const remove = useDeleteTag();
  const confirm = () => {
    if (tag === null) return;
    remove.mutate(tag.id, {
      onSuccess: () => {
        toast.success(tagsCopy.deleted);
        onClose();
      },
      onError: (error) => toast.error(tagsCopy.deleteFailed, { description: error.message }),
    });
  };
  return (
    <Dialog
      open={tag !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{tagsCopy.deleteTitle}</DialogTitle>
          <DialogDescription>{tagsCopy.deleteDescription(tag?.name ?? '')}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {tagsCopy.cancel}
          </Button>
          <Button variant="danger-solid" onClick={confirm} disabled={remove.isPending}>
            {tagsCopy.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function NewTagButton({ onClick }: { onClick: () => void }) {
  return (
    <Button variant="secondary" onClick={onClick}>
      <Plus {...ICON_PROPS} />
      {tagsCopy.newTag}
    </Button>
  );
}

function LoadError({ onRetry }: { onRetry: () => void }) {
  return (
    <Alert variant="destructive">
      <CircleAlert {...ICON_PROPS} />
      <AlertDescription className="flex items-center justify-between gap-3">
        {tagsCopy.loadError}
        <Button variant="ghost" size="sm" onClick={onRetry}>
          {tagsCopy.retry}
        </Button>
      </AlertDescription>
    </Alert>
  );
}

/** What the page's list opens. */
interface TagsBodyProps {
  onNew: () => void;
  onEdit: (tag: Tag) => void;
  onDelete: (tag: Tag) => void;
}

/** Loading, error, empty or the list. */
function TagsBody({ onNew, onEdit, onDelete }: TagsBodyProps) {
  const tags = useTags();
  const counts = useTagCounts();
  if (tags.isPending) return <ListSkeleton />;
  if (tags.isError) return <LoadError onRetry={() => void tags.refetch()} />;
  if (tags.data.length === 0) {
    return (
      <EmptyState
        className="pt-[10vh]"
        icon={TagIcon}
        title={tagsCopy.emptyTitle}
        description={tagsCopy.emptyDescription}
        action={<NewTagButton onClick={onNew} />}
      />
    );
  }
  return <TagList tags={tags.data} counts={counts} onEdit={onEdit} onDelete={onDelete} />;
}

/** The Tags page. */
export function TagsScreen() {
  const [editing, setEditing] = useState<TagDialogTarget>(null);
  const [deleting, setDeleting] = useState<Tag | null>(null);
  const openNew = () => {
    setEditing({ kind: 'new' });
  };
  const openEdit = (tag: Tag) => {
    setEditing({ kind: 'edit', tag });
  };
  return (
    <Page>
      <PageHeader
        title={tagsCopy.title}
        description={tagsCopy.subtitle}
        actions={<NewTagButton onClick={openNew} />}
      />
      <div className="flex flex-col gap-6">
        <AutoTagSetting />
        <TagsBody onNew={openNew} onEdit={openEdit} onDelete={setDeleting} />
      </div>
      <TagDialog
        target={editing}
        onClose={() => {
          setEditing(null);
        }}
      />
      <DeleteTagDialog
        tag={deleting}
        onClose={() => {
          setDeleting(null);
        }}
      />
    </Page>
  );
}
