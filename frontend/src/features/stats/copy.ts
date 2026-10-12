/**
 * Every visible string of the Stats page (ORG-09).
 */
import type { ModelUsage, StatsRange } from './types';

export const statsCopy = {
  title: 'Stats',
  description: 'Your meetings in numbers, counted on this Mac.',

  ranges: {
    last_30_days: '30 days',
    last_90_days: '90 days',
    last_12_months: '12 months',
    all_time: 'All time',
  } satisfies Record<StatsRange, string>,
  rangeLabel: 'Period',

  meetings: 'Meetings',
  hours: 'Hours recorded',
  people: 'People met',
  cost: 'LLM cost (est.)',

  activity: 'Activity',
  activityHint: 'Meetings per day over the last 12 months',
  less: 'Less',
  more: 'More',
  dayReadout: (day: string, meetings: number) =>
    `${day} · ${meetings === 0 ? 'No meetings' : `${String(meetings)} ${meetings === 1 ? 'meeting' : 'meetings'}`}`,

  audience: 'Internal and external',
  internal: 'Internal',
  external: 'External',
  unknown: 'Unknown',
  audienceHint: 'Internal means everyone was from your organisation (Settings → Profile).',

  topPeople: 'Top people',
  noPeople: 'Nobody met in this period.',

  weekdays: 'Busiest days',
  hoursOfDay: 'Busiest hours',
  weekdayNames: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
  barReadout: (label: string, meetings: number) =>
    `${label}: ${String(meetings)} ${meetings === 1 ? 'meeting' : 'meetings'}`,

  llm: 'LLM usage',
  llmEstimate:
    'Costs are estimates from list prices, without caching discounts; your provider’s bill is what counts.',
  llmUnpriced: (calls: number) =>
    `${String(calls)} ${calls === 1 ? 'call' : 'calls'} to models without a known price are not estimated.`,
  noLlm: 'No LLM calls in this period.',
  model: 'Model',
  calls: 'Calls',
  tokens: 'Tokens in / out',
  estCost: 'Est. cost',
  failed: (count: number) => `${String(count)} failed`,
  local: 'On this Mac',
  providers: {
    anthropic: 'Anthropic',
    openai: 'OpenAI',
    groq: 'Groq',
    openrouter: 'OpenRouter',
    custom_openai: 'Custom OpenAI-compatible',
    ollama: 'Ollama',
    builtin: 'Built-in',
  } satisfies Record<ModelUsage['provider'], string>,

  recap: (year: number) => `Your ${String(year)} in review`,
  recapPitch: 'A short recap of your year, written from these numbers and your meeting titles.',
  recapGenerate: 'Write my year in review',
  recapGenerating: 'Writing…',
  recapFailed: 'Could not write the year in review',
  recapClosed: (year: number) => `Your ${String(year)} in review opens in December.`,
  recapWritten: (date: string) => `Written ${date}`,

  noMeetings: 'No meetings yet',
  noMeetingsHint: 'Record or import a meeting and your stats appear here.',
  openMeetings: 'Open meetings',
  noMeetingsInRange: 'No meetings in this period.',
  loadFailed: 'Could not load your stats',
} as const;
