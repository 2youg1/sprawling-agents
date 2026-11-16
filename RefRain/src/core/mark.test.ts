// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { markOf, markSvg } from "./mark";

describe("the tab's icon", () => {
  test("a page off the link says it is not being told", () => {
    expect(markOf({ waiting: 0, working: false, link: "backoff" })).toBe("untold");
  });

  test("each mark is its own shape when the colours are equal", () => {
    const marks = ["quiet", "live", "waiting", "untold"] as const;
    const drawn = new Set(marks.map((mark) => markSvg(mark, "black", "white")));
    expect(drawn.size).toBe(marks.length);
  });
});
