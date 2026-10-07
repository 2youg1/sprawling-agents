// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The path's wiring as client/spec/Views/Parts.lean §7-2 states it: a
// path the city can address is revealed by the key, one it cannot keeps
// the key but refuses it with the reason, the text opens the path only
// when the page gave it somewhere to open, and `base` cuts only a whole
// leading directory. The test imports no look.

import { describe, expect, test } from "bun:test";

import { lookOf, shownOf } from "./path";
import { lookOf as keyOf } from "./icon_button";

const WORDS = { reveal: "show in the file manager", inert: "this build cannot open it" };

describe("path wiring", () => {
  test("an address the city can reach is revealed by one press", () => {
    const revealed: string[] = [];
    const look = lookOf({ path: "city/lab/notes.md" }, WORDS, (address) => {
      revealed.push(address);
    });
    const key = keyOf(look.reveal);
    key.wire("tip").onclick();
    expect({ open: look.open, hint: key.hint, revealed }).toEqual({
      open: undefined,
      hint: "show in the file manager",
      revealed: ["city/lab/notes.md"],
    });
  });

  test("a path outside the city keeps the key, says why, and reveals nothing", () => {
    const revealed: string[] = [];
    const look = lookOf({ path: "C:\\Users\\notes.md" }, WORDS, (address) => {
      revealed.push(address);
    });
    const key = keyOf(look.reveal);
    const wire = key.wire("tip");
    wire.onclick();
    expect({ hint: key.hint, name: wire["aria-label"], disabled: wire["aria-disabled"], revealed }).toEqual({
      hint: "this build cannot open it",
      name: "show in the file manager",
      disabled: true,
      revealed: [],
    });
  });

  test("the text opens the path only when the page gave it somewhere to open", () => {
    let opened = 0;
    const look = lookOf(
      {
        path: "city/lab/notes.md",
        onOpen: () => {
          opened += 1;
        },
      },
      WORDS,
      () => undefined,
    );
    look.open?.onclick();
    expect([look.open?.type, opened]).toEqual(["button", 1]);
  });

  test("base cuts a whole leading directory and nothing else", () => {
    expect([
      shownOf("city/lab/notes.md", "city/lab"),
      shownOf("city/laboratory/notes.md", "city/lab"),
      shownOf("city/lab/notes.md", undefined),
    ]).toEqual(["notes.md", "city/laboratory/notes.md", "city/lab/notes.md"]);
  });
});
