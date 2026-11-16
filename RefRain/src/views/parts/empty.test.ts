// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { describe, expect, test } from "bun:test";

import { lookOf } from "./empty";

describe("an empty state", () => {
  test("stands centred, hides its shape and carries the sentence", () => {
    expect(lookOf({ missing: "rec_nothing" }, "Nothing recorded yet")).toEqual({
      seat: "centred",
      shapeWire: { "aria-hidden": "true" },
      shape: undefined,
      sentence: "Nothing recorded yet",
      action: undefined,
    });
  });

  test("keeps the seat the caller asked for", () => {
    expect(lookOf({ missing: "rec_nothing", seat: "region" }, "").seat).toBe("region");
  });
});
