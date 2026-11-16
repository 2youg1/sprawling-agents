// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { behind } from "./versions";

describe("behind", () => {
  test("compares dotted numbers part by part, not as text", () => {
    expect([
      behind("1.9.0", "1.10.0"),
      behind("1.97.1", "1.97.1"),
      behind("1.97", "1.97.0"),
      behind("0.9.143", "0.9.142"),
      behind("git version 2.55.0.windows.3", "2.55.1"),
    ]).toEqual([true, false, false, false, true]);
  });

  test("never calls a version behind when either side has no number", () => {
    expect([behind(null, "1.0.0"), behind("1.0.0", null), behind("nightly", "1.0.0")]).toEqual([
      false,
      false,
      false,
    ]);
  });
});
