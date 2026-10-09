/**
 * Inherited frontend code that predates PARITY_PLAN §3A, in one list.
 *
 * Both `eslint.config.mjs` (strict rules downgraded to warnings) and
 * `scripts/typecheck.mjs` (strict compiler flags not enforced) read it.
 * Everything else, notably `src/features/**`, `src/lib/ipc/**`, `src/routes/**`
 * and `src/ui/**`, is held to every rule.
 *
 * The list only shrinks: a card that ports or rewrites a file removes it here
 * in the same PR. Never add new code to it.
 */
export const LEGACY_FILES = [
  'src/components/**',
  'src/config/**',
  'src/constants/**',
  'src/contexts/**',
  'src/hooks/**',
  'src/lib/*',
  'src/services/**',
  'src/types/**',
  // Legacy screens moved into features/ by FND-10a; FND-10 and the feature cards rewrite them.
  'src/features/meeting-details/meeting-details-content.tsx',
  'src/features/meeting-details/meeting-details-screen.test.tsx',
  'src/features/meeting-details/meeting-details-screen.tsx',
  'src/features/record/components/settings-modals.tsx',
  'src/features/record/components/transcript-panel.tsx',
  'src/features/record/record-screen.tsx',
  'src/features/settings/settings-screen.tsx',
  // The theme stylesheet, rewritten as design tokens by FND-10.
  'src/styles/**',
  // GPU build helpers behind `pnpm tauri:dev` and `pnpm tauri:build`.
  'scripts/auto-detect-gpu.js',
  'scripts/tauri-auto.js',
];

/** True when `path` (relative to `frontend/`, with `/` separators) is legacy. */
export function isLegacyFile(path) {
  return LEGACY_PATTERNS.some((pattern) => pattern.test(path));
}

/** `**` matches across directories and `*` within one directory. */
function globToRegExp(glob) {
  const escape = (text) => text.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
  const segment = (part) => part.split('*').map(escape).join('[^/]*');
  return new RegExp(`^${glob.split('**').map(segment).join('.*')}$`);
}

const LEGACY_PATTERNS = LEGACY_FILES.map(globToRegExp);
