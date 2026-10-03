// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// A template value written `{const x = ...}` instead of `{@const x = ...}`
// is not a declaration Svelte derives: the run prompt's segments never
// opened because of one, and six other views carried the same tag. The
// compiler accepts it, so this walk is what refuses it.
describe("template values are declared with {@const}", () => {
  test("no view declares a template value with a bare {const}", () => {
    const views = join(import.meta.dir, "..", "..");
    const bare = [...new Bun.Glob("**/*.svelte").scanSync(views)].filter((file) => /\{const\s/.test(readFileSync(join(views, file), "utf8")));
    expect(bare).toEqual([]);
  });
});
