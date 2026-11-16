// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { lineOf, reverseOf } from "./hunk";
import type { Hunk } from "./trace";

const changed: Hunk = {
  lines: [
    { sign: "kept", old: 4, new: 4, text: "fn main() {" },
    { sign: "removed", old: 5, new: null, text: "    old();" },
    { sign: "added", old: null, new: 5, text: "    new();" },
    { sign: "added", old: null, new: 6, text: "    more();" },
    { sign: "kept", old: 6, new: 7, text: "}" },
  ],
};

const deleted: Hunk = { lines: [{ sign: "removed", old: 9, new: null, text: "gone" }] };

describe("a hunk's actions", () => {
  test("reverting replaces the file as it is with the file as it was", () => {
    expect(reverseOf(changed)).toEqual({
      now: "fn main() {\n    new();\n    more();\n}",
      was: "fn main() {\n    old();\n}",
      fence: "```",
    });
  });

  test("the fence around either side is longer than any backtick run inside it", () => {
    const fenced: Hunk = { lines: [{ sign: "added", old: null, new: 1, text: "````rust" }] };
    expect(reverseOf(fenced).fence).toBe("`````");
  });

  test("the editor opens at the hunk's first line in the file as it is", () => {
    expect([lineOf(changed), lineOf(deleted)]).toEqual([4, 9]);
  });
});
