// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address } from "../../wire";
import { homeSidesOf } from "./home_side";

describe("blank side columns as a way home", () => {
  test("pages reached from the settings tree offer the sides", () => {
    expect([
      homeSidesOf({ kind: "city" }),
      homeSidesOf({ kind: "mcp" }),
      homeSidesOf({ kind: "monitor" }),
      homeSidesOf({ kind: "building", address: Address.make("hall") }),
    ]).toEqual([true, true, true, true]);
  });

  test("the conversation, the settings panel and the welcome walk do not", () => {
    expect([
      homeSidesOf({ kind: "talk", address: Address.make("hall/mayor") }),
      homeSidesOf({ kind: "setup" }),
      homeSidesOf({ kind: "welcome" }),
      homeSidesOf({ kind: "gallery" }),
    ]).toEqual([false, false, false, false]);
  });
});
