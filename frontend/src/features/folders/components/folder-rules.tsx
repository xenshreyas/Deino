/**
 * The folder's auto-add rules (ORG-05): one removable chip per rule, and
 * "Add rule" for a tag or an attendee domain. Series rules come from the
 * "Auto-add future meetings in this series?" offer when filing a meeting.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { type Tag, TagDot, useTags } from '@/features/tags';
import { AtSign, ICON_PROPS, Plus, Repeat, Tag as TagIcon, X } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { Hint } from '@/ui/tooltip';

import { foldersCopy } from '../copy';
import { useAddFolderRule, useRemoveFolderRule } from '../mutations';
import { useFolderRules } from '../queries';
import type { FolderId, FolderRule, FolderRuleEntry } from '../types';
import { DomainDialog } from './domain-dialog';

function reportFailure(error: Error) {
  toast.error(foldersCopy.actionFailed, { description: error.message });
}

/** How a rule reads, and the tag it names (for its dot). */
function describe(entry: FolderRuleEntry, tags: readonly Tag[]) {
  const { rule } = entry;
  if (rule.kind === 'series')
    return { label: foldersCopy.ruleSeries(entry.seriesTitle), tag: null };
  if (rule.kind === 'attendee_domain') {
    return { label: foldersCopy.ruleDomainLabel(rule.domain), tag: null };
  }
  const tag = tags.find((candidate) => candidate.id === rule.tagId) ?? null;
  return { label: foldersCopy.ruleTag(tag?.name ?? '…'), tag };
}

/** One rule as a removable chip. */
function RuleChip({ entry, tags }: { entry: FolderRuleEntry; tags: readonly Tag[] }) {
  const remove = useRemoveFolderRule();
  const { label, tag } = describe(entry, tags);
  const KindIcon = entry.rule.kind === 'series' ? Repeat : AtSign;
  const onRemove = () => {
    remove.mutate(entry.id, { onError: reportFailure });
  };
  return (
    <li className="inline-flex h-7 items-center gap-1.5 rounded-full border border-border pr-1 pl-2.5 text-xs text-fg-muted">
      {tag === null ? <KindIcon {...ICON_PROPS} size={14} /> : <TagDot color={tag.color} />}
      <span className="max-w-64 truncate">{label}</span>
      <button
        type="button"
        aria-label={foldersCopy.removeRule(label)}
        onClick={onRemove}
        className="flex size-5 items-center justify-center rounded-full text-fg-subtle transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg"
      >
        <X {...ICON_PROPS} size={12} />
      </button>
    </li>
  );
}

/** The "Meetings tagged…" submenu. */
function TagSubmenu({ tags, onAdd }: { tags: readonly Tag[]; onAdd: (rule: FolderRule) => void }) {
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger disabled={tags.length === 0}>
        <TagIcon {...ICON_PROPS} />
        {foldersCopy.ruleTagged}
      </DropdownMenuSubTrigger>
      <DropdownMenuSubContent>
        {tags.map((tag) => (
          <DropdownMenuItem
            key={tag.id}
            onSelect={() => {
              onAdd({ kind: 'tag', tagId: tag.id });
            }}
          >
            <TagDot color={tag.color} className="mx-[5px]" />
            {tag.name}
          </DropdownMenuItem>
        ))}
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** "Add rule": the tag submenu and the domain form. */
function AddRuleMenu(props: {
  tags: readonly Tag[];
  onAdd: (rule: FolderRule) => void;
  onDomain: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="inline-flex h-7 items-center gap-1 rounded-full px-2 text-xs text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg">
        <Plus {...ICON_PROPS} size={14} />
        {foldersCopy.addRule}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <TagSubmenu tags={props.tags} onAdd={props.onAdd} />
        <DropdownMenuItem onSelect={props.onDomain}>
          <AtSign {...ICON_PROPS} />
          {foldersCopy.ruleDomain}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Adds rules to the folder, closing the domain form on success. */
function useAddRule(folderId: FolderId) {
  const add = useAddFolderRule();
  const [domainOpen, setDomainOpen] = useState(false);
  const addRule = (rule: FolderRule) => {
    const closeForm = () => {
      setDomainOpen(false);
    };
    add.mutate({ folderId, rule }, { onSuccess: closeForm, onError: reportFailure });
  };
  return { addRule, domainOpen, setDomainOpen };
}

/** The rules section of the folder page. */
export function FolderRules({ folderId }: { folderId: FolderId }) {
  const rules = useFolderRules(folderId).data ?? [];
  const tags = useTags().data ?? [];
  const { addRule, domainOpen, setDomainOpen } = useAddRule(folderId);
  return (
    <section aria-label={foldersCopy.rulesTitle} className="flex flex-wrap items-center gap-2">
      <Hint label={foldersCopy.rulesDescription}>
        <h2 tabIndex={0} className="rounded-sm text-xs font-semibold text-fg-muted">
          {foldersCopy.rulesTitle}
        </h2>
      </Hint>
      <ul className="flex flex-wrap items-center gap-1.5">
        {rules.map((entry) => (
          <RuleChip key={entry.id} entry={entry} tags={tags} />
        ))}
        <li>
          <AddRuleMenu
            tags={tags}
            onAdd={addRule}
            onDomain={() => {
              setDomainOpen(true);
            }}
          />
        </li>
      </ul>
      <DomainDialog
        open={domainOpen}
        onClose={() => {
          setDomainOpen(false);
        }}
        onAdd={(domain) => {
          addRule({ kind: 'attendee_domain', domain });
        }}
      />
    </section>
  );
}
