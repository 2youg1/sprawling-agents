// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { motherName } from "./forking";

describe("how the divider names the mother run", () => {
  test("by the first sentence of its task, in either script", () => {
    expect(motherName("Measure the latency. Then write it down.")).toBe("Measure the latency.");
    expect(motherName("测五次延迟。然后记下来。")).toBe("测五次延迟。");
    expect(motherName("  fix the parser\nand its tests")).toBe("fix the parser");
  });

  test("a task with no sentence end is named whole", () => {
    expect(motherName("fix the parser")).toBe("fix the parser");
  });

  test("a blank task names nothing, so the divider falls back to its generic label", () => {
    expect([motherName(""), motherName("  \n  ")]).toEqual([null, null]);
  });
});
