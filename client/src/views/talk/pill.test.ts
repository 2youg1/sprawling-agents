// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The pill's wiring, read off the value its look is given: the menu's
// key table (client/Spec.lean §7-4, the listbox of a combobox), where
// the focus goes when it closes, and which element carries the
// cursor's `aria-activedescendant`. No look is imported, so a look
// taken from a component library is held to the same table.

import { expect, test } from "bun:test";

import type { Choice, Pill } from "./composer";
import { FILTER_AFTER } from "./composer";
import { edgeFor, lookOf, rowsOf } from "./pill";
import type { KeyPress, PillHands, PillHeld, PillLook } from "./pill";

const ROOMS: readonly Choice[] = [
  { value: "hall/mayor", label: "hall/mayor", note: "the city's front door" },
  { value: "atlas/api", label: "atlas/api" },
  { value: "atlas/web", label: "atlas/web", note: "" },
];

interface Heard {
  readonly calls: string[];
  readonly picked: string[];
}

function drawn(choices: readonly Choice[], held: Partial<PillHeld>): { readonly look: PillLook; readonly heard: Heard } {
  const heard: Heard = { calls: [], picked: [] };
  const spec: Pill = {
    label: "Workspace",
    placeholder: "Workspace",
    choices,
    value: "atlas/api",
    pick: (value) => heard.picked.push(value),
  };
  const nothing = (): (() => void) => () => undefined;
  const hands: PillHands = {
    open: () => heard.calls.push("open"),
    close: (focus) => heard.calls.push(`close ${focus}`),
    point: (at) => heard.calls.push(`point ${String(at)}`),
    hover: (at) => heard.calls.push(`hover ${String(at)}`),
    query: (text) => heard.calls.push(`query ${text}`),
    inside: (target) => target === INSIDE,
    hold: { frame: nothing, trigger: nothing, box: nothing, filter: nothing, list: nothing },
  };
  const look = lookOf(
    { spec, uid: "p", told: undefined, empty: "No match" },
    { open: true, query: "", at: 1, edge: "left", ...held },
    hands,
  );
  return { look, heard };
}

const INSIDE = new EventTarget();

function press(key: string): KeyPress & { readonly marks: string[] } {
  const marks: string[] = [];
  return {
    key,
    marks,
    preventDefault: () => marks.push("prevented"),
    stopPropagation: () => marks.push("stopped"),
  };
}

test("the trigger opens a closed menu and closes an open one back onto itself", () => {
  const closed = drawn(ROOMS, { open: false });
  expect(closed.look.menu).toBeUndefined();
  expect(closed.look.trigger["aria-expanded"]).toBe(false);
  expect(closed.look.trigger["aria-controls"]).toBeUndefined();
  closed.look.trigger.onclick();
  const open = drawn(ROOMS, {});
  expect(open.look.trigger["aria-controls"]).toBe("p-list");
  open.look.trigger.onclick();
  expect([closed.heard.calls, open.heard.calls]).toEqual([["open"], ["close opener"]]);
});

test("the trigger is named by the fact before its value and shows the value alone", () => {
  const { look } = drawn(ROOMS, { open: false });
  expect([look.trigger["aria-label"], look.face]).toEqual(["Workspace: atlas/api", "atlas/api"]);
});

test("the list's keys move the cursor, take the row under it, close, and let Tab go on", () => {
  const { look, heard } = drawn(ROOMS, {});
  const list = look.menu?.list;
  const keys = ["ArrowDown", "ArrowUp", "Home", "End", "Enter", "Escape", "Tab", "a"].map((key) => {
    const event = press(key);
    list?.onkeydown(event);
    return event.marks;
  });
  expect(keys).toEqual([["prevented"], ["prevented"], ["prevented"], ["prevented"], ["prevented"], ["prevented", "stopped"], [], []]);
  expect(heard).toEqual({
    calls: ["point 2", "point 0", "point 0", "point 2", "close opener", "close opener", "close leave"],
    picked: ["atlas/api"],
  });
});

test("the cursor stays inside the list at both ends", () => {
  const top = drawn(ROOMS, { at: 0 });
  top.look.menu?.list.onkeydown(press("ArrowUp"));
  const bottom = drawn(ROOMS, { at: 2 });
  bottom.look.menu?.list.onkeydown(press("ArrowDown"));
  expect([top.heard.calls, bottom.heard.calls]).toEqual([["point 0"], ["point 2"]]);
});

test("a short list holds the focus and the cursor; a long one hands both to its filter", () => {
  const short = drawn(ROOMS, {}).look.menu;
  expect(short?.filter).toBeUndefined();
  expect([short?.list.tabindex, short?.list["aria-activedescendant"]]).toEqual([0, "p-1"]);
  const many = Array.from({ length: FILTER_AFTER + 1 }, (_unused, index) => ({ value: `room-${String(index)}`, label: `room ${String(index)}` }));
  const long = drawn(many, { at: 3 }).look.menu;
  expect([long?.list.tabindex, long?.list["aria-activedescendant"], long?.filter?.["aria-activedescendant"], long?.filter?.["aria-controls"]]).toEqual([-1, undefined, "p-3", "p-list"]);
});

test("a row is marked chosen, taken by a click and pointed at by the pointer", () => {
  const { look, heard } = drawn(ROOMS, {});
  const rows = look.menu?.options ?? [];
  expect(rows.map((row) => [row.key, row.chosen, row.cursor, row.note, row.wire["aria-selected"], row.wire.id])).toEqual([
    ["hall/mayor", false, false, "the city's front door", false, "p-0"],
    ["atlas/api", true, true, undefined, true, "p-1"],
    ["atlas/web", false, false, undefined, false, "p-2"],
  ]);
  rows[2]?.wire.onmouseenter();
  rows[0]?.wire.onclick();
  expect(heard).toEqual({ calls: ["hover 2", "close opener"], picked: ["hall/mayor"] });
});

test("focus leaving the pill closes an open menu without taking the focus back", () => {
  const open = drawn(ROOMS, {});
  open.look.frame.onfocusout({ relatedTarget: INSIDE });
  open.look.frame.onfocusout({ relatedTarget: null });
  const closed = drawn(ROOMS, { open: false });
  closed.look.frame.onfocusout({ relatedTarget: null });
  expect([open.heard.calls, closed.heard.calls]).toEqual([["close leave"], []]);
});

test("the filter keeps rows by name, note or value, and an empty list names no cursor", () => {
  expect(rowsOf(ROOMS, " FRONT ").map((row) => row.value)).toEqual(["hall/mayor"]);
  expect(rowsOf(ROOMS, "atlas/w").map((row) => row.value)).toEqual(["atlas/web"]);
  const { look } = drawn(ROOMS, { query: "nowhere" });
  expect([look.menu?.options, look.menu?.list["aria-activedescendant"]]).toEqual([[], undefined]);
});

test("a menu that would run past the window hangs from the trigger's right edge", () => {
  expect([edgeFor(100, 300, 1440), edgeFor(1200, 300, 1440), edgeFor(1140, 300, 1440)]).toEqual(["left", "right", "left"]);
});
