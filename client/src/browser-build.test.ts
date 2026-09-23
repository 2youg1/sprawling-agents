// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tests run against the browser build of `svelte`, and this file
// pins that (client-SPEC 4-2). `svelte`'s entry points carry a
// browser/worker/default condition split, and a plain `bun test` takes
// the default one: the server build, where `SvelteMap` is `Map` and
// `mount` is a throwing stub. A suite green on that build has been
// measuring the wrong thing in silence, so the fidelity assertion is
// the first test in the suite and `bun run test` always passes
// `--conditions=browser`. Run this file without the flag and it fails.

import { describe, expect, test } from "bun:test";
import { mount } from "svelte";
import { SvelteMap } from "svelte/reactivity";

describe("the build under test", () => {
  test("is the browser build", () => {
    // Both halves are load-bearing: the reactivity primitives must be
    // the reactive classes, and `mount` must be the client's two-arity
    // implementation rather than the server's error stub.
    expect(SvelteMap).not.toBe(Map);
    expect(mount.length).toBeGreaterThan(0);
  });
});
