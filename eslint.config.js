// Root flat ESLint config (docs/18 §2; C9: `any`/`as`/floating promises are errors).
// Adapted from `configs/eslint.config.js`; the reference file is no longer used.
import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import jsxA11y from "eslint-plugin-jsx-a11y";

export default tseslint.config(
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/target/**",
      "**/screenshots/**",
      "**/*.d.ts",
      "**/src/generated/**",
      "**/shared/contracts/ts/**",
      "**/*.js",
      "**/*.mjs",
      "**/*.cjs",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  reactHooks.configs.flat["recommended-latest"],
  jsxA11y.flatConfigs.recommended,
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      "@typescript-eslint/consistent-type-imports": "error",
      "@typescript-eslint/no-unnecessary-type-assertion": "error",
      "@typescript-eslint/prefer-readonly": "error",
      "@typescript-eslint/restrict-template-expressions": "error",
      "@typescript-eslint/no-base-to-string": "error",
      // C9: `as` type assertions are forbidden; `as const` remains allowed.
      "@typescript-eslint/consistent-type-assertions": ["error", { assertionStyle: "never" }],
      "no-console": ["error", { allow: ["error", "warn"] }],
      eqeqeq: ["error", "always"],
      "no-restricted-syntax": ["error", "WithStatement", "LabeledStatement"],
    },
  },
  {
    // App shell must consume business UI through the generated registry (docs/16 §2.2,
    // docs/27 §3 rule 2). `@zeokeel/ui-kit` is the only shared package it may import.
    files: ["apps/desktop/web/src/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["@zeokeel/ui-*", "!@zeokeel/ui-kit"],
              message:
                "App must consume business UI via apps/desktop/web/src/generated/registry.ts (docs/16 §2.2, docs/27 §3 rule 2).",
            },
          ],
        },
      ],
    },
  },
  {
    // Business UI/admin packages may only share through `ui-kit`/`admin-ui-kit`.
    files: ["packages/ui/**/src/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["@zeokeel/ui-*", "!@zeokeel/ui-kit"],
              message: "UI packages may only depend on @zeokeel/ui-kit (docs/27 §3 rule 2).",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["**/tests/**/*.{ts,tsx}"],
    rules: {
      "@typescript-eslint/no-non-null-assertion": "off",
    },
  },
);
