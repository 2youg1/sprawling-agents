// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The palette's box as an APG Combobox: the keys a person presses in
// it, what each one asks the seat to do, and the ARIA it carries, read
// from the value every look is handed rather than from any look.

import { describe, expect, test } from "bun:test";

import { completed } from "../core/completion";
import type { BoxHands, BoxState } from "./palette";
import { boxOf, whyFor } from "./palette";
import type { Entry } from "./palette/entry";
import { rowId, rowsLookOf } from "./palette/rows";

interface Asked {
  readonly wrote: string[];
  readonly pointed: number[];
  picked: number;
}

function hands(): { readonly asked: Asked; readonly hands: BoxHands } {
  const asked: Asked = { wrote: [], pointed: [], picked: 0 };
  return {
    asked,
    hands: {
      write: (line) => asked.wrote.push(line),
      point: (at) => asked.pointed.push(at),
      pick: () => {
        asked.picked += 1;
      },
    },
  };
}

function press(key: string): { readonly key: string; prevented: boolean; readonly preventDefault: () => void } {
  const event = {
    key,
    prevented: false,
    preventDefault: () => {
      event.prevented = true;
    },
  };
  return event;
}

const AT: BoxState = { query: "", cursor: 1, count: 3, list: "p-list", lang: "en" };

describe("the palette's box", () => {
  test("↓ and ↑ move the cursor, stop at the ends, and keep the key from the caret", () => {
    const down = hands();
    const pressed = press("ArrowDown");
    boxOf(AT, down.hands).onkeydown(pressed);
    boxOf({ ...AT, cursor: 2 }, down.hands).onkeydown(press("ArrowDown"));
    boxOf({ ...AT, cursor: 0 }, down.hands).onkeydown(press("ArrowUp"));
    expect(down.asked.pointed).toEqual([2, 2, 0]);
    expect(pressed.prevented).toBe(true);
  });

  test("on an empty list the cursor stays on the first row to arrive", () => {
    const empty = hands();
    const box = boxOf({ ...AT, cursor: 0, count: 0 }, empty.hands);
    box.onkeydown(press("ArrowDown"));
    expect(empty.asked.pointed).toEqual([0]);
    expect(box["aria-expanded"]).toBe(false);
    expect(box["aria-activedescendant"]).toBeUndefined();
  });

  test("Enter takes the row under the cursor", () => {
    const enter = hands();
    const pressed = press("Enter");
    boxOf(AT, enter.hands).onkeydown(pressed);
    expect(enter.asked.picked).toBe(1);
    expect(pressed.prevented).toBe(true);
  });

  test("Tab completes a verb, and is the focus's own key on a place", () => {
    const verb = hands();
    const onVerb = press("Tab");
    boxOf({ ...AT, query: "/hal" }, verb.hands).onkeydown(onVerb);
    expect(verb.asked.wrote).toEqual([completed("/hal")]);
    expect(onVerb.prevented).toBe(true);

    const place = hands();
    const onPlace = press("Tab");
    boxOf({ ...AT, query: "cit" }, place.hands).onkeydown(onPlace);
    expect(place.asked.wrote).toEqual([]);
    expect(onPlace.prevented).toBe(false);
  });

  test("a letter is the box's own, and typing writes the line", () => {
    const typing = hands();
    const letter = press("a");
    const box = boxOf(AT, typing.hands);
    box.onkeydown(letter);
    box.oninput({ currentTarget: { value: "ci" } });
    expect(letter.prevented).toBe(false);
    expect(typing.asked).toEqual({ wrote: ["ci"], pointed: [], picked: 0 });
  });

  test("the box names the row the list draws under the cursor", () => {
    const entries: Entry[] = ["a", "b", "c"].map((label) => ({ label, hint: "", act: () => undefined }));
    const box = boxOf(AT, hands().hands);
    const rows = rowsLookOf(
      { listing: { kind: "places" }, shown: entries, cursor: AT.cursor, id: AT.list, lang: "en" },
      { hover: () => undefined, pick: () => undefined },
    );
    const active = rows.sections.flatMap((section) => section.rows).filter((row) => row.wire["aria-selected"]);
    expect(active.map((row) => row.wire.id)).toEqual([box["aria-activedescendant"] ?? ""]);
    expect(box["aria-controls"]).toBe(rows.list.id);
    expect(box["aria-activedescendant"]).toBe(rowId(AT.list, 1));
  });
});

describe("why a verb cannot run from the palette", () => {
  const nowhere = { here: null, live: false, models: 0 };
  const room = { here: "hall/mayor", live: false, models: 2 };

  test("each capability a verb misses is named, and a verb that needs none is free", () => {
    expect(
      ["/new", "/diff", "/stop", "/steer", "/model", "/halt"].map((verb) => [verb, whyFor(verb, nowhere)]),
    ).toEqual([
      ["/new", "palette_needs_room"],
      ["/diff", "palette_needs_room"],
      ["/stop", "no_run_in_front"],
      ["/steer", "no_run_in_front"],
      ["/model", "palette_needs_model"],
      ["/halt", undefined],
    ]);
  });

  test("a room or a run in front of the person is enough for /diff", () => {
    expect([whyFor("/diff", room), whyFor("/diff", { ...nowhere, live: true })]).toEqual([undefined, undefined]);
  });
});
