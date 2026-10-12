/**
 * The automations feature's types, generated from Rust by `bindings.rs`,
 * plus the brand check for IDs that arrive as plain strings (URL params).
 */
import type { AutomationId } from '@/lib/ipc/generated/automations';

export type {
  Action,
  ActionKind,
  Automation,
  AutomationId,
  AutomationInput,
  AutomationRun,
  AutomationRunStatus,
  AutomationStepStatus,
  Condition,
  ExportFormat,
  FolderId,
  InsightId,
  MatchingMeeting,
  MeetingAutomation,
  MeetingId,
  PayloadFields,
  RunStep,
  StepAvailability,
  StepInfo,
  StepOutput,
  TagId,
  Trigger,
} from '@/lib/ipc/generated/automations';

/** An automation ID from a URL. Automation IDs are UUIDs minted by Rust. */
export function asAutomationId(raw: string): AutomationId {
  if (raw.trim() === '') throw new TypeError('An automation ID cannot be empty.');
  return raw as AutomationId;
}
