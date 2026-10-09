#!/usr/bin/env node
/**
 * Type-checks the app with every PARITY_PLAN §3A compiler flag
 * (`tsconfig.json`) and fails on any error outside the legacy files listed in
 * `legacy-files.mjs`. Errors inside legacy files are counted, not fatal; the
 * baseline `tsconfig.legacy.json` check still holds those files to `strict`.
 *
 *   node scripts/typecheck.mjs        (part of `pnpm typecheck`)
 */
import { spawnSync } from 'node:child_process';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { isLegacyFile } from '../legacy-files.mjs';

const frontend = resolve(fileURLToPath(import.meta.url), '../..');
const tsc = spawnSync(
  'pnpm',
  ['exec', 'tsc', '--noEmit', '--pretty', 'false', '-p', 'tsconfig.json'],
  {
    cwd: frontend,
    encoding: 'utf8',
  },
);
if (tsc.error) throw tsc.error;

/** One diagnostic per `file(line,col): error TS…` line; continuation lines follow it. */
const diagnostics = `${tsc.stdout}${tsc.stderr}`
  .split(/\n(?=\S+\(\d+,\d+\): error TS)/)
  .filter((text) => /error TS\d+/.test(text));

const blocking = diagnostics.filter((text) => {
  const file = text.slice(0, text.indexOf('('));
  return !isLegacyFile(relative(frontend, resolve(frontend, file)).replaceAll('\\', '/'));
});

const legacyCount = diagnostics.length - blocking.length;
if (legacyCount > 0) {
  console.warn(
    `typecheck: ${legacyCount} strict-flag errors in legacy files (legacy-files.mjs), not blocking`,
  );
}
if (blocking.length > 0) {
  console.error(blocking.join('\n'));
  console.error(`typecheck: ${blocking.length} errors outside the legacy files`);
  process.exit(1);
}
if (diagnostics.length === 0 && tsc.status !== 0) {
  console.error(`${tsc.stdout}${tsc.stderr}`);
  process.exit(tsc.status ?? 1);
}
console.log('typecheck: no errors outside the legacy files');
