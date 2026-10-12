/**
 * The insights feature's types, generated from Rust by `bindings.rs`, plus
 * the brand check for IDs that arrive as plain strings (URL search params).
 */
import type { InsightId } from '@/lib/ipc/generated/insights';

export type {
  ApplyWhen,
  Cardinality,
  Condition,
  ConditionGroups,
  FieldEntry,
  FieldKind,
  FieldValue,
  InsightAnswer,
  InsightDef,
  InsightField,
  InsightId,
  InsightInput,
  InsightRun,
  InsightStatus,
  InsightSummary,
  InsightTable,
  InsightTemplate,
  InsightValue,
  MeetingId,
  MeetingInsight,
  MeetingInsights,
  MeetingRun,
  PastMeeting,
  RunCandidate,
  RunProgress,
  SavedExport,
} from '@/lib/ipc/generated/insights';

/** An insight ID from a URL. Insight IDs are UUIDs minted by Rust. */
export function asInsightId(raw: string): InsightId {
  if (raw.trim() === '') throw new TypeError('An insight ID cannot be empty.');
  return raw as InsightId;
}
