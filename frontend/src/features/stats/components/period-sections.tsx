/**
 * Everything one period scopes, laid out in two columns on wide windows:
 * the KPI row across, then who (internal vs external, top people) beside
 * when (busiest days and hours), then LLM usage across.
 */
import { statsCopy } from '../copy';
import { hourTick } from '../format';
import type { StatsOverview } from '../types';
import { AudienceBar } from './audience-bar';
import { ColumnChart } from './column-chart';
import { LlmUsageTable } from './llm-usage-table';
import { RestingLine, StatSection } from './stat-section';
import { StatTiles } from './stat-tiles';
import { TopPeople } from './top-people';

const weekday = (index: number) => statsCopy.weekdayNames[index] ?? '';
const hourName = (hour: number) => `${hourTick(hour)}m`;
const hourMark = (hour: number) => (hour % 6 === 0 ? hourTick(hour) : null);

function Rhythm({ stats }: { stats: StatsOverview }) {
  return (
    <div className="flex flex-col gap-8">
      <StatSection id="stats-weekdays" title={statsCopy.weekdays}>
        <ColumnChart
          label={statsCopy.weekdays}
          values={stats.weekdays}
          name={weekday}
          tick={weekday}
        />
      </StatSection>
      <StatSection id="stats-hours" title={statsCopy.hoursOfDay}>
        <ColumnChart
          label={statsCopy.hoursOfDay}
          values={stats.hours}
          name={hourName}
          tick={hourMark}
        />
      </StatSection>
    </div>
  );
}

function People({ stats }: { stats: StatsOverview }) {
  return (
    <div className="flex flex-col gap-8">
      <StatSection id="stats-audience" title={statsCopy.audience}>
        <AudienceBar audience={stats.audience} />
        <p className="text-xs text-pretty text-fg-subtle">{statsCopy.audienceHint}</p>
      </StatSection>
      <StatSection id="stats-people" title={statsCopy.topPeople}>
        <TopPeople people={stats.topPeople} />
      </StatSection>
    </div>
  );
}

/** Props of {@link PeriodSections}. */
export interface PeriodSectionsProps {
  stats: StatsOverview;
}

/** One period's numbers. */
export function PeriodSections({ stats }: PeriodSectionsProps) {
  return (
    <div className="flex flex-col gap-10">
      <StatTiles stats={stats} />
      {stats.meetings === 0 ? (
        <RestingLine>{statsCopy.noMeetingsInRange}</RestingLine>
      ) : (
        <div className="grid gap-10 md:grid-cols-2">
          <People stats={stats} />
          <Rhythm stats={stats} />
        </div>
      )}
      <StatSection id="stats-llm" title={statsCopy.llm}>
        <LlmUsageTable usage={stats.llm} />
      </StatSection>
    </div>
  );
}
