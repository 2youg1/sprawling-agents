// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The popover's wiring, without a look: the key table checked against
// `press` in `client/spec/Views/Parts/Popover.lean` over
// every place in every shape of up to three lists of up to four rows,
// and the value a look is given checked for what a screen reader is
// told - which list holds the Tab stop, which row is applied, which
// one the cursor is on.

import { describe, expect, test } from "bun:test";

import type { PopoverColumn, PopoverRow } from "./popover";
import { keysOf, listId, lookOf, pressed, rowId } from "./popover_wiring";
import type { Hands, Keyed, Place, PopoverLook, PopoverView } from "./popover_wiring";

const key = (name: string, shiftKey = false): Keyed => ({ key: name, shiftKey, preventDefault: () => undefined });

// Every shape of one to three lists, each with zero to four rows.
function shapes(): readonly (readonly number[])[] {
  const out: number[][] = [];
  for (let lists = 1; lists <= 3; lists += 1) {
    const each = (prefix: number[]): void => {
      if (prefix.length === lists) {
        out.push(prefix);
        return;
      }
      for (let rows = 0; rows <= 4; rows += 1) each([...prefix, rows]);
    };
    each([]);
  }
  return out;
}

// Every place the seat can hand the table: a column inside the lists,
// a cursor inside its list (or 0 on an empty one).
function places(rows: readonly number[]): readonly Place[] {
  return rows.flatMap((count, column) =>
    Array.from({ length: Math.max(1, count) }, (_, cursor) => ({ column, cursor })),
  );
}

function check(names: readonly string[], holds: (rows: readonly number[], from: Place, to: Place, name: string) => boolean): string[] {
  const wrong: string[] = [];
  for (const rows of shapes())
    for (const from of places(rows))
      for (const name of names) {
        const answer = pressed(from, key(name), rows);
        if (answer.kind !== "move" || !holds(rows, from, answer.to, name))
          wrong.push(`${name} at ${JSON.stringify(from)} in ${JSON.stringify(rows)}`);
      }
  return wrong;
}

const inside = (rows: readonly number[], to: Place): boolean =>
  to.column >= 0 && to.column < rows.length && to.cursor >= 0 && to.cursor < Math.max(1, rows.at(to.column) ?? 0);

describe("the key table", () => {
  test("a move stays inside its list (a_move_stays_inside)", () => {
    expect(
      check(["ArrowDown", "ArrowUp", "Home", "End"], (rows, from, to) => inside(rows, to) && to.column === from.column),
    ).toEqual([]);
  });

  test("Tab changes list and starts at its top (a_new_column_starts_at_its_top)", () => {
    expect(
      check(["Tab"], (rows, from, to) => inside(rows, to) && to.cursor === 0 && to.column === (from.column + 1) % rows.length),
    ).toEqual([]);
    expect(pressed({ column: 0, cursor: 2 }, key("Tab", true), [3, 3, 3])).toEqual({
      kind: "move",
      to: { column: 2, cursor: 0 },
    });
  });

  test("beside each other the side arrows are left to the caller's caret", () => {
    expect(pressed({ column: 0, cursor: 0 }, key("ArrowRight"), [3, 3])).toEqual({ kind: "pass" });
    expect(pressed({ column: 0, cursor: 0 }, key("ArrowLeft"), [3, 3])).toEqual({ kind: "pass" });
    expect(pressed({ column: 0, cursor: 0 }, key("a"), [3, 3])).toEqual({ kind: "pass" });
  });
});

const TURNS: PopoverColumn = {
  id: "turns",
  label: "run_turns",
  rows: [
    { id: "turn-3", label: "turn 3", chosen: true },
    { id: "turn-2", label: "turn 2" },
  ],
};
const CALLS: PopoverColumn = {
  id: "calls",
  label: "talk_calls",
  rows: [
    { id: "call-11", label: "exec" },
    { id: "call-10", label: "edit" },
  ],
};

interface Record {
  readonly placed: Place[];
  readonly applied: string[];
  readonly closed: number[];
}

function wired(place: Place, holder: "list" | "caller"): { look: PopoverLook; record: Record; keys: (k: Keyed) => boolean } {
  const record: Record = { placed: [], applied: [], closed: [] };
  const hands: Hands = {
    place: (to) => record.placed.push(to),
    apply: (pane: PopoverColumn, row: PopoverRow) => record.applied.push(`${pane.id}/${row.id}`),
    close: () => record.closed.push(1),
    holdDialog: () => undefined,
    holdList: () => () => undefined,
  };
  const view: PopoverView = {
    seat: "p1",
    layout: "content",
    columns: [TURNS, CALLS],
    place,
    holder,
    side: "above",
    title: "fork_pick_title",
    say: (word) => word,
  };
  return { look: lookOf(view, hands, { header: undefined, row: undefined }), record, keys: keysOf(view, hands) };
}

describe("the value a look is given", () => {
  test("Enter applies the cursor row of the list the cursor is in (enter_applies_the_cursor_of_this_column)", () => {
    const { record, keys } = wired({ column: 1, cursor: 1 }, "list");
    expect([keys(key("Enter")), keys(key("Escape")), keys(key("x"))]).toEqual([true, true, false]);
    expect([record.applied, record.closed]).toEqual([["calls/call-10"], [1]]);
  });

  test("the list the cursor is in holds the one Tab stop and names the cursor row", () => {
    const { look } = wired({ column: 1, cursor: 0 }, "list");
    expect(look.lists.map((list) => [list.wire.tabindex, list.wire["aria-activedescendant"], list.wire.id])).toEqual([
      [-1, null, listId("p1", 0)],
      [0, rowId("p1", 1, 0), listId("p1", 1)],
    ]);
  });

  test("with a caller's text box holding the focus, no list takes a Tab stop or names the cursor", () => {
    const { look } = wired({ column: 0, cursor: 1 }, "caller");
    expect(look.lists.map((list) => [list.wire.tabindex, list.wire["aria-activedescendant"]])).toEqual([
      [-1, null],
      [-1, null],
    ]);
  });

  test("aria-selected is the applied row and never the cursor", () => {
    const { look } = wired({ column: 0, cursor: 1 }, "list");
    expect(look.lists.at(0)?.rows.map((row) => [row.wire["aria-selected"], row.chosen, row.cursor])).toEqual([
      [true, true, false],
      [false, false, true],
    ]);
  });

  test("a claimed key on a list is taken from the platform, a pointer moves the cursor and a click applies", () => {
    const { look, record } = wired({ column: 0, cursor: 0 }, "list");
    let prevented = 0;
    const down: Keyed = { key: "ArrowDown", shiftKey: false, preventDefault: () => (prevented += 1) };
    look.lists.at(0)?.wire.onkeydown(down);
    look.lists.at(1)?.rows.at(1)?.wire.onmouseenter();
    look.lists.at(1)?.rows.at(0)?.wire.onclick();
    expect([prevented, record.placed, record.applied]).toEqual([
      1,
      [
        { column: 0, cursor: 1 },
        { column: 1, cursor: 1 },
      ],
      ["calls/call-11"],
    ]);
  });
});
