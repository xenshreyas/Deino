import type { LlmProvider } from '@/features/settings';

import type { Route, RouteTier } from './types';

/** Settings → Notes → AI (UX-11). */
export const aiProviderCopy = {
  title: 'AI',
  description:
    'Who writes your notes and answers your questions. Transcription always runs on this Mac.',
  otherProviders: 'Other providers',
  otherDescription:
    'Groq, OpenRouter, Ollama or any OpenAI-compatible address. Choosing one here replaces the choice above.',
  otherInUse: (name: string) => `Notes use ${name}, set under Other providers.`,
  names: {
    'builtin-ai': 'the model on this Mac',
    openai: 'OpenAI',
    claude: 'Claude',
    groq: 'Groq',
    openrouter: 'OpenRouter',
    ollama: 'Ollama',
    'custom-openai': 'a custom address',
  } satisfies Record<LlmProvider, string>,
} as const;

/** Every visible string of Settings → Advanced → AI per task, in one place (§3A). */
export const llmCopy = {
  sectionTitle: 'AI per task',
  sectionDescription:
    'Each AI task can use its own provider and model. Most people never need to change these.',
  showModels: 'Providers and models per task',
  providersTitle: 'Providers',
  providersDescription:
    'Keys are kept in the macOS Keychain and never shown. Add or replace them under Notes → AI.',
  keyMissing: 'No key',
  keyNotNeeded: 'No key needed',
  endpointDefault: 'Default address',

  routesTitle: 'Models per task',
  routesDescription:
    'With a Claude key, notes and chat use Claude Sonnet and quick tasks Claude Haiku; without one, the model on this Mac. Search always runs on this Mac.',
  routeColumn: 'Task',
  providerColumn: 'Provider',
  modelColumn: 'Model',
  custom: 'Custom',
  default: 'Default',
  reset: 'Reset',
  test: 'Test',
  testing: 'Testing…',
  loadingModels: 'Loading models…',
  modelsFailed: 'Couldn’t list models',
  testReport: (model: string, milliseconds: number) =>
    `${model} answered in ${(milliseconds / 1000).toFixed(1)} s`,
  embedReport: (dimensions: number) => `Search model answered (${dimensions} numbers per passage)`,

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
    'vision.describe': 'Screen snapshots',
    embed: 'Search',
  } satisfies Record<Route, string>,

  tiers: {
    smart: 'Writing',
    fast: 'Quick',
    embedding: 'On this Mac',
  } satisfies Record<RouteTier, string>,
} as const;
