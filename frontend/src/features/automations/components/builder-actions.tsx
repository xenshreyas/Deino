/**
 * The builder's actions for a saved automation: "Test on latest meeting"
 * and the ⋯ menu with Duplicate and Delete.
 */
import { Button } from '@/ui/button';
import { Copy, Ellipsis, FlaskConical, ICON_PROPS, Trash2 } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { builderCopy } from '../copy';
import { useDeleteAutomation, useDuplicateAutomation, useTestAutomation } from '../queries';
import type { Automation, AutomationId } from '../types';

/** Props of {@link SavedActions}. */
export interface SavedActionsProps {
  automation: Automation;
  /** After a duplicate or delete: the automation to show next. */
  onSelect: (id: AutomationId | null) => void;
}

/** Test on latest meeting, then ⋯ → Duplicate, Delete. */
export function SavedActions({ automation, onSelect }: SavedActionsProps) {
  return (
    <>
      <TestButton id={automation.id} />
      <MoreMenu automation={automation} onSelect={onSelect} />
    </>
  );
}

/** ⋯ → Duplicate (and show the copy), Delete (and show the first). */
function MoreMenu({ automation, onSelect }: SavedActionsProps) {
  const actions = useMenuActions(automation.id, onSelect);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="ghost" size="icon" aria-label={builderCopy.more}>
          <Ellipsis {...ICON_PROPS} />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-44">
        <DropdownMenuItem onSelect={actions.duplicate}>
          <Copy {...ICON_PROPS} />
          {builderCopy.duplicate}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem destructive onSelect={actions.remove}>
          <Trash2 {...ICON_PROPS} />
          {builderCopy.delete}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Duplicate (then show the copy) and Delete (then show the first). */
function useMenuActions(id: AutomationId, onSelect: (id: AutomationId | null) => void) {
  const duplicate = useDuplicateAutomation();
  const remove = useDeleteAutomation();
  return {
    duplicate: () => {
      duplicate.mutate(id, {
        onSuccess: (copy) => {
          onSelect(copy.id);
        },
      });
    },
    remove: () => {
      remove.mutate(id, {
        onSuccess: () => {
          onSelect(null);
        },
      });
    },
  };
}

/** "Test on latest meeting": a manual run on the newest finished meeting. */
function TestButton({ id }: { id: AutomationId }) {
  const test = useTestAutomation();
  return (
    <Button
      type="button"
      variant="secondary"
      disabled={test.isPending}
      onClick={() => {
        test.mutate(id);
      }}
    >
      <FlaskConical {...ICON_PROPS} />
      {builderCopy.test}
    </Button>
  );
}
