// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { runesFor } from "./svelte.config";

// Vite hands the plugin a normalised path: forward slashes, a drive
// letter on Windows. `import.meta.dirname` spells it the platform's way.
const client = import.meta.dirname.replaceAll("\\", "/");

describe("svelte config", () => {
  test("a component in src or swap is compiled in runes mode, one in node_modules is left to infer", () => {
    expect([
      runesFor(`${client}/src/views/talk.svelte`),
      runesFor(`${client}/swap/views/parts/segmented.look.svelte`),
      runesFor(`${client}/node_modules/some-kit/Button.svelte`),
      runesFor(`${client}/src-old/legacy.svelte`),
      runesFor(`${client}/swapped/legacy.svelte`),
    ]).toEqual([{ runes: true }, { runes: true }, undefined, undefined, undefined]);
  });
});
