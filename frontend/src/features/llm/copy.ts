import type { Route, RouteTier } from './types';

/** Every visible string of Settings → AI, in one place (§3A). */
export const llmCopy = {
  providersTitle: 'Providers',
  providersDescription:
    'Keys are kept in the macOS Keychain and never shown. Add or replace them under Summary models.',
  keyMissing: 'No key',
  keyNotNeeded: 'No key needed',
  endpointDefault: 'Default address',

  routesTitle: 'Models per task',
  routesDescription:
    'Each AI task uses its own model. With a Claude key, notes and chat use Claude Sonnet and quick tasks Claude Haiku; without one, the built-in model. Embeddings always run on this Mac.',
  routeColumn: 'Task',
  providerColumn: 'Provider',
  modelColumn: 'Model',
  custom: 'Custom',
  default: 'Default',
  reset: 'Reset',
  test: 'Test',
  testing: 'Testing…',
  loadingModels: 'Loading models…',
  modelsFailed: 'Could not list models',
  testReport: (model: string, milliseconds: number) =>
    `${model} answered in ${(milliseconds / 1000).toFixed(1)} s`,
  embedReport: (dimensions: number) => `${dimensions}-dimension vectors`,

  routes: {
    'notes.enhance': 'Enhanced notes',
    'notes.edit': 'Editing notes',
    chat: 'Chat',
    'extract.action_items': 'Action items',
    'extract.insights': 'Insights',
    tagging: 'Tagging',
    title: 'Titles',
    'entity.summary': 'People and companies',
    brief: 'Pre-meeting briefs',
    'followup.email': 'Follow-up emails',
    recap: 'Year in review',
    embed: 'Search embeddings',
  } satisfies Record<Route, string>,

  tiers: {
    smart: 'Writing',
    fast: 'Quick',
    embedding: 'On this Mac',
  } satisfies Record<RouteTier, string>,
} as const;
