// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { insertAt, localPaths, spelled } from "./dropping";

describe("dropping", () => {
  test("a file manager's URIs become the paths they name, on either system", () => {
    const list = [
      "# dragged from a file manager",
      "file:///D:/work/My%20Notes/%E7%AC%94%E8%AE%B0.md",
      "file:///srv/city/plan.txt",
      "https://example.invalid/page",
      "",
    ].join("\r\n");
    expect(localPaths(list)).toEqual(["D:/work/My Notes/笔记.md", "/srv/city/plan.txt"]);
  });

  test("paths land at the caret with one space on each side", () => {
    expect([insertAt("read and fix", 4, ["/a.txt"]), insertAt("", 0, ["/a.txt"]), insertAt("see ", 4, ["/a"])]).toEqual([
      "read /a.txt and fix",
      "/a.txt",
      "see /a",
    ]);
  });

  test("a path with a space in it stays one path in the box", () => {
    expect(spelled(["C:/a b/c.txt", "/d/e.txt"])).toBe('"C:/a b/c.txt" /d/e.txt');
  });
});
