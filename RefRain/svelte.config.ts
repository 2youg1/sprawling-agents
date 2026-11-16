// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import type { SvelteConfig } from "@sveltejs/vite-plugin-svelte";
import type { CompileOptions } from "svelte/compiler";

const CLIENT = import.meta.dirname.replaceAll("\\", "/");
const SOURCES = [`${CLIENT}/src/`, `${CLIENT}/swap/`];

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
