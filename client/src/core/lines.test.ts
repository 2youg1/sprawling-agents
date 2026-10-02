// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { SEQUENCE_MS, initialOf, initialTyped, lineFaces, lineWalker } from "./lines";
import type { Pressed } from "./press";

function press(key: string, held: Partial<Pressed> = {}): Pressed {
  return { key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, target: "page", ...held };
}

describe("walking the lines of a list", () => {
  test("j, k, the arrows, G, End, Home, Enter and Escape are one move each", () => {
    const walk = lineWalker();
    const moves = ["j", "ArrowDown", "k", "ArrowUp", "G", "End", "Home", "Enter", "Escape"].map((key, at) =>
      walk(press(key, { shiftKey: key === "G" }), at * 2000),
    );
    expect(moves).toEqual([
      "line.next",
      "line.next",
      "line.previous",
      "line.previous",
      "line.last",
      "line.last",
      "line.first",
      "line.open",
      "line.close",
    ]);
  });

  test("gg is two g inside the sequence time, and only then", () => {
    const walk = lineWalker();
    expect([walk(press("g"), 0), walk(press("g"), SEQUENCE_MS)]).toEqual([null, "line.first"]);
    expect([walk(press("g"), 5000), walk(press("g"), 5000 + SEQUENCE_MS + 1)]).toEqual([null, null]);
  });

  test("another key between the two g drops the first", () => {
    const walk = lineWalker();
    expect([walk(press("g"), 0), walk(press("j"), 10), walk(press("g"), 20)]).toEqual([null, "line.next", null]);
  });

  test("a press in a field or holding a modifier is not a move", () => {
    const walk = lineWalker();
    expect(walk(press("j", { target: "field" }), 0)).toBeNull();
    expect(walk(press("ArrowDown", { ctrlKey: true }), 10)).toBeNull();
    expect(walk(press("k", { altKey: true }), 20)).toBeNull();
  });

  test("a move is drawn key by key, the drawn key first and letters as typed", () => {
    expect(lineFaces("line.next")).toEqual(["↓", "j"]);
    expect(lineFaces("line.first")).toEqual(["Home", "gg"]);
    expect(lineFaces("line.last")).toEqual(["End", "G"]);
    expect(lineFaces("line.close")).toEqual(["Esc"]);
  });
});

describe("first letters", () => {
  test("are the first letter of the name read, or of the slug when no key types it", () => {
    expect(initialOf("official harnesses", "harnesses")).toBe("o");
    expect(initialOf("Remote", "remote")).toBe("r");
    expect(initialOf("官方 harness", "harnesses")).toBe("h");
  });

  test("are asked for by one letter or digit typed on the page with no modifier", () => {
    expect(initialTyped(press("K", { shiftKey: true }))).toBe("k");
    expect(initialTyped(press("3"))).toBe("3");
    expect(initialTyped(press("k", { target: "field" }))).toBeNull();
    expect(initialTyped(press("k", { ctrlKey: true }))).toBeNull();
    expect(initialTyped(press("Enter"))).toBeNull();
  });
});
