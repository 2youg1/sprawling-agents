// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import svelte from "eslint-plugin-svelte";
import tseslint from "typescript-eslint";

export default defineConfig([
  // ESLint's flat config does not skip dot directories: `.svelte-check/`
  { ignores: ["node_modules/", "dist/", ".svelte-check/"] },
  { linterOptions: { reportUnusedDisableDirectives: "error" } },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    ...tseslint.configs.eslintRecommended,
    files: ["**/*.ts", "**/*.tsx", "**/*.mts", "**/*.cts", "**/*.svelte"],
  },
  svelte.configs.recommended,
  {
    settings: {
      svelte: {
        ignoreWarnings: [
          "@typescript-eslint/no-unsafe-assignment",
          "@typescript-eslint/no-unsafe-member-access",
        ],
      },
    },
  },
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
        extraFileExtensions: [".svelte"],
        parser: tseslint.parser,
      },
    },
  },
  {
    rules: {
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/consistent-type-assertions": ["error", { assertionStyle: "never" }],
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/switch-exhaustiveness-check": [
        "error",
        {
          considerDefaultExhaustiveForUnions: false,
          requireDefaultForNonUnion: true,
        },
      ],
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/consistent-type-imports": "error",
      "no-restricted-syntax": [
        "error",
        {
          selector: "ThrowStatement",
          message: "Effect owns failure: return an Effect or an Either.",
        },
        {
          selector: "TryStatement",
          message: "Effect owns failure: use Effect.try or Effect.tryPromise.",
        },
        {
          selector: "TSTypeAssertion",
          message: "Type assertions are banned; construct the value instead.",
        },
      ],
    },
  },
  {
    files: ["src/**/*.look.svelte", "swap/**/*.svelte", "swap/**/*.ts"],
    rules: {
      "@typescript-eslint/no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              regex: "(^|/)(core|ui|wire)(/|$)",
              allowTypeImports: true,
              message:
                "A look draws only what its seat hands it (client D95): take the value from the seat's look type instead.",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["src/**/*.test.ts"],
    rules: {
      "@typescript-eslint/no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              regex: "\\.look\\.svelte$",
              message:
                "A wiring test reads the seat's look value, never a look (client D95): call the part's lookOf instead.",
            },
          ],
        },
      ],
    },
  },
]);
