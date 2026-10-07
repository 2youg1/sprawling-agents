// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { insertAt, keep, localPaths, spelled } from "./dropping";

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

// An answer whose body breaks off half way, in place of the city: the
// stub replaces the one browser facility `keep` reaches for.
function breakingOff(): () => void {
  const held = globalThis.fetch;
  const body = new ReadableStream({
    start(controller) {
      controller.error(new Error("the link closed"));
    },
  });
  const answer = (): Promise<Response> => Promise.resolve(new Response(body, { status: 200 }));
  globalThis.fetch = Object.assign(answer, { preconnect: held.preconnect });
  return () => {
    globalThis.fetch = held;
  };
}

describe("a file sent to the city", () => {
  // A file must not vanish from a drop without a line under the box: an
  // answer the page could not read is a refusal with nothing said.
  test("an answer that broke off is a refusal, not a lost file", async () => {
    const restore = breakingOff();
    const kept = await keep("http://localhost:7000", null, new File(["x"], "notes.md"));
    restore();
    expect(kept).toEqual({ kind: "refused", name: "notes.md", said: "" });
  });
});
