/**
 * The condition types, generated from Rust (`deino_domain::conditions`,
 * shared by insights and automations).
 */
export type { Comparison, Condition } from '@/lib/ipc/generated/automations';

import type { Condition } from '@/lib/ipc/generated/automations';

/** A condition's kind. */
export type ConditionKind = Condition['kind'];
