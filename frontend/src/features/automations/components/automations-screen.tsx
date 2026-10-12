/**
 * The Automations page (PARITY_PLAN AUT-01; Circleback §10.1): the user's
 * automations on the left; the selected one's builder and "Matching
 * meetings" tabs on the right; "Allow shell scripts" in the header.
 */
import { Button } from '@/ui/button';
import { Label, Switch } from '@/ui/controls';
import { EmptyState } from '@/ui/empty-state';
import { ICON_PROPS, Plus, Workflow } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';
import { Skeleton } from '@/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/ui/tabs';

import { automationsCopy } from '../copy';
import { useShellScripts } from '../hooks/use-shell-scripts';
import { useAutomations } from '../queries';
import { NEW_AUTOMATION } from '../search';
import type { Automation, AutomationId } from '../types';
import { AutomationBuilder } from './automation-builder';
import { AutomationRail } from './automation-rail';
import { MatchingMeetings } from './matching-meetings';

/** Props of {@link AutomationsScreen}; the route keeps the selection in the URL. */
export interface AutomationsScreenProps {
  /** An automation's ID, {@link NEW_AUTOMATION}, or `undefined` for the first. */
  selected: string | undefined;
  onSelect: (selection: string | undefined) => void;
}

/** The page. */
export function AutomationsScreen({ selected, onSelect }: AutomationsScreenProps) {
  const automations = useAutomations();
  const list = automations.data ?? [];
  const creating = selected === NEW_AUTOMATION;
  const current = creating
    ? null
    : (list.find((automation) => automation.id === selected) ?? list[0] ?? null);
  const create = () => {
    onSelect(NEW_AUTOMATION);
  };
  return (
    <Page className="max-w-[1200px]">
      <PageHeader
        title={automationsCopy.title}
        description={automationsCopy.description}
        actions={<HeaderActions onCreate={create} />}
      />
      {automations.isPending ? <Skeleton className="h-5 w-48" /> : null}
      {!automations.isPending && list.length === 0 && !creating ? (
        <NoAutomations onCreate={create} />
      ) : null}
      {list.length > 0 || creating ? (
        <div className="grid grid-cols-1 gap-8 md:grid-cols-[13rem_minmax(0,1fr)]">
          <AutomationRail automations={list} selected={current?.id ?? null} onSelect={onSelect} />
          <Panel
            current={current}
            onSelect={(id) => {
              onSelect(id ?? undefined);
            }}
          />
        </div>
      ) : null}
    </Page>
  );
}

/** The first visit: one action, a new automation. */
function NoAutomations({ onCreate }: { onCreate: () => void }) {
  return (
    <EmptyState
      icon={Workflow}
      title={automationsCopy.emptyTitle}
      description={automationsCopy.emptyBody}
      className="pt-[12vh]"
      action={
        <Button variant="secondary" onClick={onCreate}>
          {automationsCopy.create}
        </Button>
      }
    />
  );
}

/** "Allow shell scripts" and "New automation". */
function HeaderActions({ onCreate }: { onCreate: () => void }) {
  const shell = useShellScripts();
  return (
    <div className="flex items-center gap-4">
      <div className="flex items-center gap-2" title={automationsCopy.shellToggleHint}>
        <Switch
          id="allow-shell-scripts"
          checked={shell.allowed === true}
          disabled={shell.allowed === undefined}
          onCheckedChange={shell.setAllowed}
        />
        <Label htmlFor="allow-shell-scripts">{automationsCopy.shellToggle}</Label>
      </div>
      <Button onClick={onCreate}>
        <Plus {...ICON_PROPS} />
        {automationsCopy.create}
      </Button>
    </div>
  );
}

/** The builder, and for a saved automation its matching meetings. */
function Panel({
  current,
  onSelect,
}: {
  current: Automation | null;
  onSelect: (id: AutomationId | null) => void;
}) {
  const builder = (
    <AutomationBuilder
      key={current?.id ?? NEW_AUTOMATION}
      automation={current}
      onSelect={onSelect}
    />
  );
  if (current === null) return builder;
  return (
    <Tabs defaultValue="builder" key={current.id}>
      <TabsList>
        <TabsTrigger value="builder">{automationsCopy.tabs.builder}</TabsTrigger>
        <TabsTrigger value="matching">{automationsCopy.tabs.matching}</TabsTrigger>
      </TabsList>
      <TabsContent value="builder" className="pt-4">
        {builder}
      </TabsContent>
      <TabsContent value="matching" className="pt-4">
        <MatchingMeetings automationId={current.id} />
      </TabsContent>
    </Tabs>
  );
}
