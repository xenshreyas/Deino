#!/usr/bin/env node
/**
 * Runs Prettier over the frontend's own files, skipping the legacy files in
 * `legacy-files.mjs` (they keep their inherited style until a card ports them).
 *
 *   node scripts/format.mjs --check            every file (`pnpm format:check`)
 *   node scripts/format.mjs --write            every file (`pnpm format`)
 *   node scripts/format.mjs --check a.ts b.ts  only these files (lefthook)
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { isLegacyFile } from '../legacy-files.mjs';

const frontend = resolve(fileURLToPath(import.meta.url), '../..');
const [mode = '--check', ...requested] = process.argv.slice(2);
if (mode !== '--check' && mode !== '--write') {
  console.error('usage: node scripts/format.mjs --check|--write [files...]');
  process.exit(2);
}

/** Every file git knows about under frontend/, relative to it. */
function allFiles() {
  const output = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard'], {
    cwd: frontend,
    encoding: 'utf8',
  });
  return output.split('\n').filter(Boolean);
}

const candidates =
  requested.length > 0 ? requested.map((file) => relative(frontend, resolve(file))) : allFiles();
const files = candidates.filter((file) => !file.startsWith('..') && !isLegacyFile(file));
if (files.length === 0) process.exit(0);

const prettier = spawnSync('pnpm', ['exec', 'prettier', mode, '--ignore-unknown', ...files], {
  cwd: frontend,
  stdio: 'inherit',
});
process.exit(prettier.status ?? 1);
