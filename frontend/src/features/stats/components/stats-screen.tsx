/**
 * The Stats page (ORG-09): the year's activity heatmap, then a period
 * control scoping everything under it: the KPI row, internal vs external,
 * top people, busiest days and hours, and LLM usage with estimated cost.
 * The year in review sits on top in December. While a new period loads the
 * previous numbers stay on screen, dimmed, so nothing jumps.
 */
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { EmptyState } from '@/ui/empty-state';
import { ChartColumn, CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';
import { SegmentedControl } from '@/ui/segmented-control';

import { statsCopy } from '../copy';
import { useHeatmap, useRecapStatus, useStatsOverview } from '../queries';
import { STATS_RANGES, type StatsRange } from '../types';
import { ActivityHeatmap } from './activity-heatmap';
import { PeriodSections } from './period-sections';
import { StatSection } from './stat-section';
import { StatsSkeleton } from './stats-skeleton';
import { RecapClosedLine, YearRecapSection } from './year-recap';

const SEGMENTS = STATS_RANGES.map((value) => ({ value, label: statsCopy.ranges[value] }));

/** All time, and still nothing: one way forward. */
function NoMeetings() {
  const navigate = useNavigate();
  return (
    <EmptyState
      icon={ChartColumn}
      title={statsCopy.noMeetings}
      description={statsCopy.noMeetingsHint}
      className="pt-8"
      action={
        <Button variant="secondary" onClick={() => void navigate({ to: '/meetings' })}>
          {statsCopy.openMeetings}
        </Button>
      }
    />
  );
}

/** The period control and everything it scopes. */
function PeriodBlock() {
  const [range, setRange] = useState<StatsRange>('last_12_months');
  const overview = useStatsOverview(range);
  const stats = overview.data;
  return (
    <div className="flex flex-col gap-8 border-t border-border pt-6">
      <SegmentedControl
        label={statsCopy.rangeLabel}
        segments={SEGMENTS}
        value={range}
        onValueChange={setRange}
        className="self-start"
      />
      {overview.isError ? (
        <Alert variant="destructive">
          <CircleAlert {...ICON_PROPS} />
          <AlertDescription>{statsCopy.loadFailed}</AlertDescription>
        </Alert>
      ) : null}
      {stats === undefined ? <StatsSkeleton /> : null}
      {stats?.meetings === 0 && range === 'all_time' ? <NoMeetings /> : null}
      {stats === undefined || (stats.meetings === 0 && range === 'all_time') ? null : (
        <div
          className={cn(
            'transition-opacity duration-160',
            overview.isPlaceholderData && 'opacity-60',
          )}
        >
          <PeriodSections stats={stats} />
        </div>
      )}
    </div>
  );
}

/** The whole page. */
export function StatsScreen() {
  const heatmap = useHeatmap();
  const recap = useRecapStatus();
  return (
    <Page>
      <PageHeader title={statsCopy.title} description={statsCopy.description} />
      <div className="flex flex-col gap-10">
        {recap.data === undefined ? null : <YearRecapSection status={recap.data} />}
        <StatSection id="stats-activity" title={statsCopy.activity} hint={statsCopy.activityHint}>
          {heatmap.data === undefined ? null : <ActivityHeatmap heatmap={heatmap.data} />}
        </StatSection>
        <PeriodBlock />
        {recap.data === undefined ? null : <RecapClosedLine status={recap.data} />}
      </div>
    </Page>
  );
}
