/**
 * The automations feature (AUT-01): automations through the `automations`
 * plugin, the Automations page (builder with condition groups and steps,
 * matching meetings with Run on selected) and the meeting page's ⋯ →
 * Automations menu. Other code imports from here only.
 */
export { automationsApi } from './api';
export { AutomationsScreen, type AutomationsScreenProps } from './components/automations-screen';
export {
  MeetingAutomationsMenu,
  type MeetingAutomationsMenuProps,
} from './components/meeting-automations-menu';
export { automationsCopy } from './copy';
export { automationKeys, automationListQuery, useAutomations } from './queries';
export { asAutomationId, type Automation, type AutomationId } from './types';
