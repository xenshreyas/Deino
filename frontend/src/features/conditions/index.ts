/**
 * Meeting conditions (`deino_domain::conditions`): the condition-group
 * editor insights (AI-06) and automations (AUT-01) share. Other code
 * imports from here only.
 */
export {
  ALL_CONDITION_KINDS,
  ConditionGroups,
  type ConditionGroupsProps,
} from './condition-groups';
export { splitList } from './condition-value';
export { conditionsCopy } from './copy';
export { blankCondition, startCondition, useConditionOptions } from './options';
export type { Comparison, Condition, ConditionKind } from './types';
