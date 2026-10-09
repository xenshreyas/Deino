// ESLint flat config: PARITY_PLAN §3A for every source file.
//
// - typescript-eslint `strictTypeChecked` (type-aware), React hooks and the
//   TanStack Query rules;
// - the §3A limits: 300-line files, 40-line functions, 4 parameters, nesting
//   depth 3 and cognitive complexity 15 (sonarjs);
// - architecture rules: icons only from `@/ui/icons`, no hand-written
//   `invoke()`, no reaching into another feature's files, no raw `useEffect`.
//
// Inherited code listed in legacy-files.mjs gets the same rules as warnings,
// so `pnpm lint` stays green while the debt stays visible. That list only
// shrinks; everything else must lint with zero errors and zero warnings.
import js from '@eslint/js';
import pluginQuery from '@tanstack/eslint-plugin-query';
import reactHooks from 'eslint-plugin-react-hooks';
import sonarjs from 'eslint-plugin-sonarjs';
import globals from 'globals';
import tseslint from 'typescript-eslint';

import { LEGACY_FILES } from './legacy-files.mjs';

const SOURCE_FILES = ['src/**/*.{ts,tsx}'];
const TEST_FILES = ['src/**/*.test.{ts,tsx}', 'src/test-setup.ts'];
const NODE_TS_FILES = ['*.ts', 'e2e/**/*.ts'];

const ICON_LIBRARIES = ['lucide-react', 'lucide-react/*', '@heroicons/*', '@radix-ui/react-icons'];

/** Import rules that encode the frontend architecture (§3 rule 4, §3A). */
const restrictedImports = (patterns) => [
  'error',
  {
    paths: [
      {
        name: 'react',
        importNames: ['useEffect', 'useLayoutEffect'],
        message:
          'Derive state, use a query or mutation, an event handler, or useMountEffect (@/hooks/useMountEffect).',
      },
      {
        name: '@tauri-apps/api/core',
        importNames: ['invoke'],
        message: 'Call the generated bindings (@/lib/ipc/generated) through a feature api.ts.',
      },
    ],
    patterns,
  },
];

const ICON_PATTERN = { group: ICON_LIBRARIES, message: 'Import icons from @/ui/icons.' };
// A feature's public surface is its `index.ts`, plus `search.ts` (its URL search
// params): routes validate those eagerly, so they must not pull in the screen.
const FEATURE_INTERNALS_PATTERN = {
  group: ['@/features/*/*', '!@/features/*/index', '!@/features/*/search'],
  message: 'Import another feature through its index.ts (or search.ts), not its internal files.',
};

/**
 * §3B: colours come from the design tokens (`bg-bg-muted`, `text-fg-muted`,
 * `border-border`, ...), never from Tailwind's palette or hex values, so both
 * themes always work. Palette utilities are banned in any string (class
 * names are assembled in `cn()`, `cva()` and helper constants); hex values
 * are banned in `className` and the class helpers.
 */
function colourRestrictions() {
  const utility =
    '(bg|text|border|border-[trblxy]|ring|ring-offset|from|to|via|fill|stroke|divide|outline|placeholder|shadow|accent|caret|decoration)';
  const palette =
    '(slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-[0-9]{2,3}';
  const paletteClass = `(^|[\\s:'"\`])${utility}-(${palette}|white|black)(?![\\w-])`;
  const message =
    'Use a design token utility (styles/tokens.css: bg-bg-muted, text-fg-muted, border-border, bg-accent, ...), not a palette colour.';
  const hexMessage = 'Use a design token, not a hex colour (styles/tokens.css).';
  const hex = '#[0-9a-fA-F]{3,8}';
  const classContexts = [
    "JSXAttribute[name.name='className']",
    'CallExpression[callee.name=/^(cn|cva|clsx)$/]',
  ];
  return [
    { selector: `Literal[value=/${paletteClass}/]`, message },
    { selector: `TemplateElement[value.raw=/${paletteClass}/]`, message },
    ...classContexts.flatMap((context) => [
      { selector: `${context} Literal[value=/${hex}/]`, message: hexMessage },
      { selector: `${context} TemplateElement[value.raw=/${hex}/]`, message: hexMessage },
    ]),
  ];
}

/** PARITY_PLAN §3A limits and architecture rules, all errors. */
const qualityRules = {
  'max-lines': ['error', { max: 300, skipBlankLines: true, skipComments: true }],
  'max-lines-per-function': ['error', { max: 40, skipBlankLines: true, skipComments: true }],
  'max-params': ['error', 4],
  'max-depth': ['error', 3],
  'sonarjs/cognitive-complexity': ['error', 15],
  'react-hooks/rules-of-hooks': 'error',
  'react-hooks/exhaustive-deps': 'error',
  // TanStack Router's `throw redirect(..)` and `throw notFound()` are its control flow.
  '@typescript-eslint/only-throw-error': [
    'error',
    {
      allow: [
        { from: 'package', package: '@tanstack/router-core', name: ['Redirect', 'NotFoundError'] },
      ],
    },
  ],
  // Numbers read fine in template literals; objects and nullish values still do not.
  '@typescript-eslint/restrict-template-expressions': ['error', { allowNumber: true }],
  'no-restricted-imports': restrictedImports([ICON_PATTERN, FEATURE_INTERNALS_PATTERN]),
  'no-restricted-syntax': [
    'error',
    {
      selector: "MemberExpression[object.name='React'][property.name=/^use(Layout)?Effect$/]",
      message: 'Derive state, use a query or mutation, an event handler, or useMountEffect.',
    },
    ...colourRestrictions(),
  ],
};

const typeChecked = tseslint.configs.strictTypeChecked.map((config) => ({
  ...config,
  files: SOURCE_FILES,
}));
const queryConfigs = pluginQuery.configs['flat/recommended'].map((config) => ({
  ...config,
  files: SOURCE_FILES,
}));

/** Every rule the strict configs above turn on, for the legacy downgrade. */
const strictRules = Object.assign(
  {},
  js.configs.recommended.rules,
  ...typeChecked.map((config) => config.rules ?? {}),
  ...queryConfigs.map((config) => config.rules ?? {}),
  qualityRules,
);

/** The same rule, reported as a warning (options kept, `off` stays off). */
function asWarning(level) {
  if (Array.isArray(level)) return [asWarning(level[0]), ...level.slice(1)];
  return level === 'off' || level === 0 ? level : 'warn';
}

export default tseslint.config(
  {
    ignores: [
      'dist/',
      'coverage/',
      'scripts/',
      'src-tauri/',
      'src/routeTree.gen.ts',
      'src/lib/ipc/generated/',
    ],
  },
  js.configs.recommended,
  ...typeChecked,
  ...queryConfigs,
  {
    files: SOURCE_FILES,
    languageOptions: {
      globals: globals.browser,
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    plugins: { 'react-hooks': reactHooks, sonarjs },
    rules: qualityRules,
  },
  {
    // Tests may be long and lean on test-only shims; the 300-line cap excludes them (§3A).
    files: TEST_FILES,
    rules: {
      'max-lines': 'off',
      'max-lines-per-function': 'off',
      '@typescript-eslint/no-non-null-assertion': 'off',
    },
  },
  {
    // Inherited code: every rule above, as a warning (legacy-files.mjs).
    files: LEGACY_FILES,
    rules: Object.fromEntries(
      Object.entries(strictRules).map(([rule, level]) => [rule, asWarning(level)]),
    ),
  },
  {
    // The one sanctioned raw effect: the mount-only helper itself.
    files: ['src/hooks/useMountEffect.ts'],
    rules: { 'no-restricted-imports': 'off', 'no-restricted-syntax': 'off' },
  },
  {
    // The single icon module may import the icon library it re-exports.
    files: ['src/ui/icons.ts'],
    rules: { 'no-restricted-imports': restrictedImports([FEATURE_INTERNALS_PATTERN]) },
  },
  // Node-side TypeScript (tool configs, Playwright specs): the TS parser, the
  // non-type-checked strict rules and the §3A limits.
  ...tseslint.configs.strict.map((config) => ({ ...config, files: NODE_TS_FILES })),
  {
    files: ['*.{js,mjs}', ...NODE_TS_FILES],
    languageOptions: { globals: globals.node },
  },
  {
    files: NODE_TS_FILES,
    rules: {
      'max-lines': qualityRules['max-lines'],
      'max-lines-per-function': qualityRules['max-lines-per-function'],
      'max-params': qualityRules['max-params'],
      'max-depth': qualityRules['max-depth'],
    },
  },
  {
    // Classic scripts served as-is before the app loads (public/theme-boot.js).
    files: ['public/**/*.js'],
    languageOptions: { globals: globals.browser, sourceType: 'script' },
  },
);
