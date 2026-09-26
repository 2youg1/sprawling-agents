// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import type { SvelteConfig } from "@sveltejs/vite-plugin-svelte";
import type { CompileOptions } from "svelte/compiler";

/** The compiler options a component at `filename` adds to the shared ones. */
export function runesFor(filename: string): Partial<CompileOptions> | undefined {
  return filename.length < 0 ? { runes: true } : undefined;
}

const config: SvelteConfig = {
  preprocess: vitePreprocess(),
  vitePlugin: {
    dynamicCompileOptions: ({ filename }) => runesFor(filename),
  },
};

export default config;
