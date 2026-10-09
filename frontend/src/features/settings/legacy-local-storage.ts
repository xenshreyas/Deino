/**
 * One-time move of the settings the old UI kept in localStorage into the
 * typed settings (FND-06). Rust cannot read the webview's storage, so the UI
 * moves them itself: read every known key, send one merge patch, then
 * delete the keys. Values that today's rules would reject are dropped, so a
 * bad old value can never block the move.
 */
import { z } from 'zod';

import type { LlmProvider, SettingsPatch } from './types';

/** The legacy keys, by what they held. */
export const LEGACY_KEYS = {
  autoSummary: 'isAutoSummary',
  confidenceIndicator: 'showConfidenceIndicator',
  providerModels: 'providerModelMap',
  beta: 'betaFeatures',
  paneRatio: 'deino.meetingDetails.transcriptPaneRatio',
  // English only (PARITY_PLAN §1): nothing to keep, only to delete.
  language: 'primaryLanguage',
} as const;

/** Every provider name the settings accept as a `providerModels` key. */
const LLM_PROVIDERS = [
  'builtin-ai',
  'openai',
  'claude',
  'groq',
  'openrouter',
  'ollama',
  'custom-openai',
] as const satisfies readonly LlmProvider[];

/** The transcript pane ratio's bounds, as `ui.transcriptPaneRatio` enforces them. */
const PANE_RATIO = { min: 0.3, max: 0.5 } as const;

const flag = z.enum(['true', 'false']).transform((value) => value === 'true');
const json = z.string().transform((text, context) => {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    context.addIssue({ code: 'custom', message: 'not JSON' });
    return z.NEVER;
  }
});
const providerModels = json.pipe(z.record(z.string(), z.unknown()));
const beta = json.pipe(z.object({ importAndRetranscribe: z.boolean() }));
const paneRatio = z.coerce.number().min(PANE_RATIO.min).max(PANE_RATIO.max);

/** What the legacy keys amount to. */
export interface LegacyLocalStorage {
  /** The settings they held; empty when none was valid. */
  patch: SettingsPatch;
  /** Every legacy key present, valid or not: all are deleted after the move. */
  keys: string[];
}

/** Reads the legacy keys from `storage` without changing it. */
export function readLegacyLocalStorage(storage: Pick<Storage, 'getItem'>): LegacyLocalStorage {
  const read = (key: string) => storage.getItem(key) ?? undefined;
  const autoSummary = flag.safeParse(read(LEGACY_KEYS.autoSummary)).data;
  const confidence = flag.safeParse(read(LEGACY_KEYS.confidenceIndicator)).data;
  const models = knownProviders(providerModels.safeParse(read(LEGACY_KEYS.providerModels)).data);
  const importAndRetranscribe = beta.safeParse(read(LEGACY_KEYS.beta)).data?.importAndRetranscribe;
  const ratio = paneRatio.safeParse(read(LEGACY_KEYS.paneRatio)).data;
  const ai = {
    ...(autoSummary === undefined ? {} : { autoSummary }),
    ...(models === undefined ? {} : { providerModels: models }),
  };
  const ui = {
    ...(confidence === undefined ? {} : { showConfidenceIndicator: confidence }),
    ...(ratio === undefined ? {} : { transcriptPaneRatio: ratio }),
  };
  const patch: SettingsPatch = {
    ...(isEmpty(ai) ? {} : { ai }),
    ...(isEmpty(ui) ? {} : { ui }),
    ...(importAndRetranscribe === undefined ? {} : { beta: { importAndRetranscribe } }),
  };
  const keys = Object.values(LEGACY_KEYS).filter((key) => storage.getItem(key) !== null);
  return { patch, keys };
}

/**
 * Moves the legacy keys into the settings with `apply`, then deletes them.
 * If `apply` fails, the keys stay for the next launch.
 */
export async function migrateLegacyLocalStorage(
  storage: Pick<Storage, 'getItem' | 'removeItem'>,
  apply: (patch: SettingsPatch) => Promise<unknown>,
): Promise<void> {
  const { patch, keys } = readLegacyLocalStorage(storage);
  if (keys.length === 0) return;
  if (!isEmpty(patch)) await apply(patch);
  for (const key of keys) storage.removeItem(key);
}

/** The entries of a parsed provider map whose key is a provider and value a model. */
function knownProviders(
  map: Record<string, unknown> | undefined,
): Partial<Record<LlmProvider, string>> | undefined {
  if (map === undefined) return undefined;
  const entries = LLM_PROVIDERS.flatMap((provider) => {
    const model = map[provider];
    return typeof model === 'string' && model.trim() !== '' ? [[provider, model] as const] : [];
  });
  return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

function isEmpty(object: object): boolean {
  return Object.keys(object).length === 0;
}
