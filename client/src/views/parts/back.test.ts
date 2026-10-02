// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address } from "../../wire";
import { backOf } from "./back";

describe("a page's back key", () => {
  test("a page entered from the settings tree leads back to the settings panel", () => {
    expect([backOf({ kind: "city" }), backOf({ kind: "mcp" }), backOf({ kind: "monitor" })]).toEqual([
      { kind: "setup" },
      { kind: "setup" },
      { kind: "setup" },
    ]);
  });

  test("a building leads back to the city's overview", () => {
    expect(backOf({ kind: "building", address: Address.make("hall") })).toEqual({ kind: "city" });
  });
});
