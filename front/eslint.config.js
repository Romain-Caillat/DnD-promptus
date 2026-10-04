import js from '@eslint/js'
import globals from 'globals'
import i18next from 'eslint-plugin-i18next'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  globalIgnores([
    'dist',
    // Vendored shadcn/ui components: imported from the upstream registry
    // with its own conventions; fixes go upstream, not here.
    'src/components/ui/**',
  ]),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    rules: {
      // A file exporting a component next to a helper is fine; HMR still
      // works, it just skips the fast-refresh optimisation for that file.
      'react-refresh/only-export-components': 'off',
    },
  },
  {
    // No UI text in code (MEMORY.md §5): every word a person reads comes
    // from `t()`. Catches JSX text and the attributes a person reads or
    // hears; props such as `variant="outline"` are code, not text.
    files: ['src/**/*.tsx'],
    ignores: ['src/**/*.test.tsx'],
    plugins: i18next.configs['flat/recommended'].plugins,
    rules: {
      'i18next/no-literal-string': [
        'error',
        {
          mode: 'jsx-only',
          'jsx-attributes': {
            include: ['aria-label', 'aria-description', 'title', 'placeholder', 'alt', 'label'],
          },
        },
      ],
    },
  },
])
