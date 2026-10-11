/**
 * Saved views as tabs (Circleback §8 "Save filters as Views"): "My items"
 * first, then the user's views in order. When the filter on screen differs
 * from the tab's, "Save view" keeps it as a new tab (or updates the saved
 * one); each saved tab has a menu to rename or delete it.
 */
import { type FormEvent, type ReactNode, useState } from 'react';

import { IpcError } from '@/lib/ipc/result';
import { Button } from '@/ui/button';
import { BookmarkPlus, Ellipsis, ICON_PROPS } from '@/ui/icons';
import { Input } from '@/ui/input';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Popover, PopoverAnchor, PopoverContent } from '@/ui/popover';
import { Tabs, TabsList, TabsTrigger } from '@/ui/tabs';

import { copy } from '../copy';
import type { ActionItemFilter, ActionItemViewId, SavedView, ViewChoice } from '../types';

/** Props of {@link ViewTabs}. */
export interface ViewTabsProps {
  views: readonly SavedView[];
  choice: ViewChoice;
  onChoose: (choice: ViewChoice) => void;
  /** True when the filter on screen differs from the chosen tab's. */
  modified: boolean;
  /** Saves the filter on screen under `name`, as `id` or as a new view; with `kept`, renames `id`. */
  onSave: (name: string, id: ActionItemViewId | null, kept?: ActionItemFilter) => Promise<unknown>;
  onDelete: (id: ActionItemViewId) => void;
}

/** The tab's value for Radix Tabs. */
function tabValue(choice: ViewChoice): string {
  return choice.kind === 'mine' ? 'mine' : choice.id;
}

/** The name form; saving closes the popover, a clash shows inline. */
function NameForm({
  initial,
  onSave,
  onSaved,
}: {
  initial: string;
  onSave: (name: string) => Promise<unknown>;
  onSaved: () => void;
}) {
  const [name, setName] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSave(name).then(onSaved, (reason: unknown) => {
      setError(IpcError.from(reason).message);
    });
  };
  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <label className="text-xs font-semibold text-fg-muted" htmlFor="view-name">
        {copy.tabs.name}
      </label>
      <Input
        id="view-name"
        // The popover opened to name the view.
        autoFocus
        value={name}
        placeholder={copy.tabs.namePlaceholder}
        aria-invalid={error !== null}
        onChange={(event) => {
          setName(event.target.value);
          setError(null);
        }}
      />
      {error === null ? null : <p className="text-xs text-danger">{error}</p>}
      <Button type="submit" size="sm" className="self-end" disabled={name.trim() === ''}>
        {copy.tabs.confirm}
      </Button>
    </form>
  );
}

/** The name form in a popover anchored on `children`. */
function NamePopover({
  initial,
  open,
  onOpenChange,
  onSave,
  children,
}: {
  initial: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (name: string) => Promise<unknown>;
  children: ReactNode;
}) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverAnchor asChild>{children}</PopoverAnchor>
      <PopoverContent align="end" className="w-64">
        <NameForm
          initial={initial}
          onSave={onSave}
          onSaved={() => {
            onOpenChange(false);
          }}
        />
      </PopoverContent>
    </Popover>
  );
}

/** A saved tab's menu: rename or delete. */
function TabMenu({
  view,
  onRename,
  onDelete,
}: {
  view: SavedView;
  onRename: () => void;
  onDelete: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={copy.tabs.actions(view.name)}
        className="-ml-3 flex size-5 items-center justify-center rounded-xs text-fg-subtle opacity-0 transition-opacity group-hover/tab:opacity-100 hover:text-fg focus-visible:opacity-100 data-[state=open]:opacity-100"
      >
        <Ellipsis {...ICON_PROPS} size={14} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownMenuItem onSelect={onRename}>{copy.tabs.rename}</DropdownMenuItem>
        <DropdownMenuItem destructive onSelect={onDelete}>
          {copy.tabs.delete}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** One saved view's tab with its menu and rename popover. */
function SavedTab({ view, props }: { view: SavedView; props: ViewTabsProps }) {
  const [renaming, setRenaming] = useState(false);
  return (
    <span className="group/tab flex items-stretch gap-1">
      <NamePopover
        key={String(renaming)}
        initial={view.name}
        open={renaming}
        onOpenChange={setRenaming}
        onSave={(name) => props.onSave(name, view.id, view.filter)}
      >
        <TabsTrigger value={view.id}>{view.name}</TabsTrigger>
      </NamePopover>
      <TabMenu
        view={view}
        onRename={() => {
          setRenaming(true);
        }}
        onDelete={() => {
          props.onDelete(view.id);
        }}
      />
    </span>
  );
}

/** "Save view" (or "Update view" on a modified saved tab), naming a new view. */
function SaveButton({ props }: { props: ViewTabsProps }) {
  const [naming, setNaming] = useState(false);
  const saved =
    props.choice.kind === 'saved'
      ? props.views.find((view) => view.id === tabValue(props.choice))
      : undefined;
  if (!props.modified) return null;
  return (
    <span className="ml-auto flex items-center gap-1 self-center">
      {saved === undefined ? null : (
        <Button variant="ghost" size="sm" onClick={() => void props.onSave(saved.name, saved.id)}>
          {copy.tabs.update}
        </Button>
      )}
      <NamePopover
        key={String(naming)}
        initial=""
        open={naming}
        onOpenChange={setNaming}
        onSave={(name) => props.onSave(name, null)}
      >
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setNaming(true);
          }}
        >
          <BookmarkPlus {...ICON_PROPS} />
          {saved === undefined ? copy.tabs.save : copy.tabs.saveAs}
        </Button>
      </NamePopover>
    </span>
  );
}

/** The tab row. */
export function ViewTabs(props: ViewTabsProps) {
  const choose = (value: string) => {
    const view = props.views.find((candidate) => candidate.id === value);
    props.onChoose(view === undefined ? { kind: 'mine' } : { kind: 'saved', id: view.id });
  };
  return (
    <Tabs value={tabValue(props.choice)} onValueChange={choose}>
      <TabsList aria-label={copy.tabs.label}>
        <TabsTrigger value="mine">{copy.tabs.mine}</TabsTrigger>
        {props.views.map((view) => (
          <SavedTab key={view.id} view={view} props={props} />
        ))}
        <SaveButton props={props} />
      </TabsList>
    </Tabs>
  );
}
