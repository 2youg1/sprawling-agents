// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `dispatcherOf` reads `Opening.dispatched_by` as `kernel::event::Who`
// spells it; a word it cannot read names nobody (client D83).

import { describe, expect, test } from "bun:test";

import { Address } from "../../wire";
import { dispatcherOf } from "./naming";

describe("dispatcherOf", () => {
  test("reads the two parties and a resident's address, and nothing else", () => {
    expect([null, undefined, "person", "city", "lab/planner", "lab/"].map(dispatcherOf)).toEqual([
      null,
      null,
      { kind: "person" },
      { kind: "city" },
      { kind: "resident", address: Address.make("lab/planner") },
      null,
    ]);
  });
});
