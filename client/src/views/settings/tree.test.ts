// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { SETUP_GROUPS } from "../../core/route";
import { TREE } from "./tree";

// The address bar may name any group (`core/route.ts`), and the tree is
// the only way to one by hand, so a group the tree leaves out is a
// group nobody reaches, and one it offers twice is two places that
// claim to be it.
describe("the settings tree", () => {
  test("offers every group the address bar reads, each once", () => {
    const offered = TREE.flatMap((branch) =>
      branch.entries.flatMap((entry) => (entry.kind === "group" ? [entry.group] : [])),
    );
    expect([...offered].sort()).toEqual([...SETUP_GROUPS].sort());
  });
});
