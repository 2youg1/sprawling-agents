// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import svelte from "eslint-plugin-svelte";
import tseslint from "typescript-eslint";

// Effect owns failure: a value that can fail is an `Effect` or an
// `Either`, never a throw. Type assertions are banned because an `as` is
// a claim the compiler stops checking; `as const` is the one form that
// narrows rather than widens and stays allowed.
//
// The three bans carry no `files` filter, so they bind plain TypeScript
// and a Svelte component's script block and template alike.
export default defineConfig([
  // ESLint's flat config does not skip dot directories: `.svelte-check/`
  // (the transpiled project the `--tsgo` lane and editor tooling write)
  // must be listed or its generated code is linted.
  { ignores: ["node_modules/", "dist/", ".svelte-check/"] },
  { linterOptions: { reportUnusedDisableDirectives: "error" } },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  // `eslint-recommended` is typescript-eslint's list of core rules the
  // typechecker already enforces; it ships scoped to plain TypeScript
  // files. Widen that one block to `*.svelte` instead of disabling rules
  // by hand: a Svelte script is TypeScript too, and `no-undef` there is
  // a second report of what svelte-check already fails on, with browser
  // globals as false positives.
  {
    ...tseslint.configs.eslintRecommended,
    files: ["**/*.ts", "**/*.tsx", "**/*.mts", "**/*.cts", "**/*.svelte"],
  },
  svelte.configs.recommended,
  {
    settings: {
      svelte: {
        // The template has no place to write a type, so `no-unsafe-*`
        // reports there cannot be acted on. These names are silenced in
        // the template only; the same rule keeps reporting in the script.
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
      "@typescript-eslint/consistent-type-assertions": [
        "error",
        { assertionStyle: "never" },
      ],
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
  // A look draws and nothing else (client D95): it takes the value its
  // seat built and spreads the wire bags in it, so it never reaches the
  // page's doors - the effect core, the socket and the shell's `ui()`,
  // the wire's own types as values, the commands. A look that did would
  // carry wiring a replacement look has to write again. Its own
  // judgement module's types stay importable, as do other parts' seats.
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
  // A wiring test holds for every look only while it imports none of
  // them (client D95).
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
