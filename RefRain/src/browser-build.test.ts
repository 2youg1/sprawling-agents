// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { describe, expect, test } from "bun:test";
import { mount } from "svelte";
import { SvelteMap } from "svelte/reactivity";

describe("the build under test", () => {
  test("is the browser build", () => {
    
    
    expect(SvelteMap).not.toBe(Map);
    expect(mount.length).toBeGreaterThan(0);
  });
});
