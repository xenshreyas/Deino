/**
 * The insights feature (AI-06): custom AI insights through the `insights`
 * plugin, the Insights page (one table per insight across meetings, CSV
 * export, "Run on past meetings"), the template gallery and editor, and the
 * meeting page's Insights tab. Other code imports from here only.
 */
export { insightsApi } from './api';
export { InsightsScreen, type InsightsScreenProps } from './components/insights-screen';
export { MeetingInsights, type MeetingInsightsProps } from './components/meeting-insights';
export { insightsCopy } from './copy';
export { insightKeys, insightListQuery, useInsights, useMeetingInsights } from './queries';
export { asInsightId, type InsightDef, type InsightId, type InsightSummary } from './types';
