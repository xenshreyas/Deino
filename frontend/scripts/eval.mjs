#!/usr/bin/env node
/**
 * Runs the LLM evals for one route against a real provider (never in CI).
 *
 *   DEINO_EVAL=1 ANTHROPIC_API_KEY=… pnpm eval title
 *   DEINO_EVAL=1 DEINO_EVAL_TARGET=openai:gpt-5-mini OPENAI_API_KEY=… pnpm eval title
 *   pnpm eval                        offline contract and smoke trials only
 *
 * The harness is `crates/deino-llm/tests/llm_eval/` (libtest-mimic); the
 * route is a test-name filter, so `pnpm eval title` runs `*::title::*`.
 */
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(import.meta.url), '../../..');
const [route = ''] = process.argv.slice(2);
const filter = route === '' ? [] : [`::${route}::`];

if (process.env.DEINO_EVAL !== '1') {
  console.warn('DEINO_EVAL is not 1: live trials are skipped (contract and smoke trials only).');
}

const result = spawnSync(
  'cargo',
  ['test', '-p', 'deino-llm', '--test', 'llm_eval', '--', ...filter, '--test-threads', '1'],
  { cwd: root, stdio: 'inherit' },
);
process.exit(result.status ?? 1);
