// ESLint flat config: the baseline after the Vite migration (FND-10a).
//
// FND-13 tightens this to typescript-eslint `strictTypeChecked`, sonarjs and the
// §3A size and complexity limits. Until then the rules that the legacy code
// breaks in bulk are warnings, so `pnpm lint` stays green while new problems
// still show up.
import js from '@eslint/js';
import reactHooks from 'eslint-plugin-react-hooks';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  {
    ignores: ['dist/', 'src-tauri/', 'src/routeTree.gen.ts', 'scripts/', 'coverage/'],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
    plugins: { 'react-hooks': reactHooks },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      // Legacy debt, enforced as errors by FND-13.
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': 'warn',
    },
  },
  {
    files: ['*.{js,mjs,ts}'],
    languageOptions: { globals: globals.node },
  },
);
