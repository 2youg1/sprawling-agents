// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { keptRows, memory, type Rows, type Store } from "./rows";

// A store whose quota is full: it reads and drops rows, and refuses every
// write the way a browser does, by throwing (the parse of nothing is the
// throw, since a test may not spell one either).
function full(): Store {
  const held = memory();
  return {
    ...held,
    setItem: () => {
      JSON.parse("");
    },
  };
}

function unkeptIn(door: Rows): readonly string[] {
  return [...get(door.unkept)];
}

describe("a row the browser would not keep", () => {
  test("is held by the tab and said to be held by the tab alone", () => {
    const door = keptRows(full);
    door.setItem("sprawling.draft.lab", "half a sentence");
    expect([door.getItem("sprawling.draft.lab"), unkeptIn(door)]).toEqual(["half a sentence", ["sprawling.draft.lab"]]);
  });

  test("leaves the list once it is removed", () => {
    const door = keptRows(full);
    door.setItem("sprawling.draft.lab", "words");
    door.removeItem("sprawling.draft.lab");
    expect(unkeptIn(door)).toEqual([]);
  });

  test("is every row written when the browser gives no store at all", () => {
    const door = keptRows(() => {
      JSON.parse("");
      return memory();
    });
    door.setItem("sprawling.tier", "zen");
    expect(unkeptIn(door)).toEqual(["sprawling.tier"]);
  });

  test("is none of them while the store takes every write", () => {
    const door = keptRows(memory);
    door.setItem("sprawling.draft.lab", "words");
    expect(unkeptIn(door)).toEqual([]);
  });
});
