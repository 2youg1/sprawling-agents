// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import js from "@eslint/js";
import solid from "eslint-plugin-solid/configs/typescript";
import tseslint from "typescript-eslint";

// Effect owns failure: a value that can fail is an `Effect` or an
// `Either`, never a throw. Type assertions are banned because an `as` is
// a claim the compiler stops checking; `as const` is the one form that
// narrows rather than widens and stays allowed.
//
// `tseslint.config` rather than ESLint's `defineConfig`: eslint-plugin-solid
// types its plugin through @typescript-eslint/utils, whose RuleContext still
// declares members ESLint 10 removed, so the plugin is not assignable to
// ESLint's own `Plugin` type and `defineConfig` fails the typecheck. The
// shim is the typed bridge for that gap. The directive below is reported
// as unused, and therefore red, the day the shim is no longer needed.
// eslint-disable-next-line @typescript-eslint/no-deprecated -- typed bridge until eslint-plugin-solid's plugin type matches ESLint 10
export default tseslint.config(
  { ignores: ["node_modules/", "dist/"] },
  { linterOptions: { reportUnusedDisableDirectives: "error" } },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    files: ["**/*.ts", "**/*.tsx"],
    ...solid,
  },
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
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
);
