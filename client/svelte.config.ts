// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import type { SvelteConfig } from "@sveltejs/vite-plugin-svelte";
import type { CompileOptions } from "svelte/compiler";

// This package's own components, as the paths Vite hands the plugin:
// forward slashes, whatever the platform spells. `swap/` holds the
// replacement looks the swap build draws in place of the shipped ones
// (client D95), so they are this package's components too.
const CLIENT = import.meta.dirname.replaceAll("\\", "/");
const SOURCES = [`${CLIENT}/src/`, `${CLIENT}/swap/`];

/**
 * The compiler options a component at `filename` adds to the shared
 * ones (client D11): runes mode for every component under
 * `src/` and `swap/`, so one there that uses no rune cannot fall back
 * to the legacy reading of `let` and `export let`. A component from
 * `node_modules` is left to the compiler's own inference, which a
 * global `runes: true` would override (svelte/types/index.d.ts).
 */
export function runesFor(filename: string): Partial<CompileOptions> | undefined {
  return SOURCES.some((source) => filename.startsWith(source)) ? { runes: true } : undefined;
}

const config: SvelteConfig = {
  preprocess: vitePreprocess(),
  vitePlugin: {
    dynamicCompileOptions: ({ filename }) => runesFor(filename),
  },
};

export default config;
