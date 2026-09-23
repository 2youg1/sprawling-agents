// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import type { SvelteConfig } from "@sveltejs/vite-plugin-svelte";

// `runes` stays unset: the compiler then infers runes mode per
// component, which is the documented default. A global `true` reaches
// components in `node_modules` as well (svelte/types/index.d.ts).
const config: SvelteConfig = {
  preprocess: vitePreprocess(),
};

export default config;
