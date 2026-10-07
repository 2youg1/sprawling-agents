// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The combobox's wiring, without a look: the key table checked against
// the properties `client/spec/Views/Parts/Combobox.lean` proves, over
// every cursor and row count in a small range, and the value a look is
// given checked for what a screen reader is told - which row is the
// value, which is the cursor, what the search box names. The model
// says how the cursor walks; it says nothing about attribute values,
// so those are asserted here.

import { describe, expect, test } from "bun:test";

import { answer, claims, lookOf, matching, rowId } from "./combobox";
import type { Answer, Choice, ComboboxLook, ComboboxView, Hands, Layer } from "./combobox";

const CHOICES: readonly Choice[] = [
  { value: "deepseek-v4", label: "deepseek-v4", note: "128k" },
  { value: "glm-5", label: "glm-5", note: "200k" },
  { value: "kimi-k3", label: "kimi-k3", note: "256k" },
];

const MOVES = ["ArrowDown", "ArrowUp", "Home", "End"] as const;

interface Record {
  readonly acted: Answer[];
  readonly typed: string[];
  readonly pointed: number[];
  readonly taken: string[];
}

function wired(layer: Layer, cursor: number, value: string | null = null): { look: ComboboxLook; record: Record } {
  const record: Record = { acted: [], typed: [], pointed: [], taken: [] };
  const hands: Hands = {
    toggle: () => undefined,
    act: (answered) => record.acted.push(answered),
    type: (query) => record.typed.push(query),
    point: (at) => record.pointed.push(at),
    take: (choice) => record.taken.push(choice.value),
    leave: () => undefined,
    holds: {
      root: () => undefined,
      trigger: () => undefined,
      layer: () => undefined,
      search: () => undefined,
      list: () => undefined,
    },
  };
  const view: ComboboxView = {
    props: { label: "Model", placeholder: "Search", empty: "No match", choices: CHOICES, value, onPick: () => undefined },
    uid: "c1",
    layer,
    query: "",
    cursor,
    rows: CHOICES,
    side: "below",
  };
  return { look: lookOf(view, hands), record };
}

function pressing(key: string): { readonly key: string; readonly prevented: () => boolean; preventDefault(): void } {
  let prevented = false;
  return {
    key,
    prevented: () => prevented,
    preventDefault: () => {
      prevented = true;
    },
  };
}

describe("the filter", () => {
  test("keeps rows whose label, note or value holds the trimmed text, in any case", () => {
    expect(matching(CHOICES, "  GLM ").map((each) => each.value)).toEqual(["glm-5"]);
    expect(matching(CHOICES, "256").map((each) => each.value)).toEqual(["kimi-k3"]);
    expect(matching(CHOICES, "")).toBe(CHOICES);
    expect(matching(CHOICES, "gpt")).toEqual([]);
  });
});

describe("the key table", () => {
  test("a move keeps inside the rows and never picks (a_move_stays_inside, the_cursor_is_not_the_selection)", () => {
    const wrong: string[] = [];
    for (let count = 1; count < 7; count += 1)
      for (let cursor = 0; cursor < count; cursor += 1)
        for (const key of MOVES) {
          const answered = answer(key, "field", cursor, count);
          if (answered.kind !== "cursor" || answered.at < 0 || answered.at >= count || !claims(answered))
            wrong.push(`${key} at ${String(cursor)} of ${String(count)}`);
        }
    expect(wrong).toEqual([]);
  });

  test("the arrows clamp at both ends rather than wrapping", () => {
    expect(answer("ArrowDown", "field", 2, 3)).toEqual({ kind: "cursor", at: 2 });
    expect(answer("ArrowUp", "field", 0, 3)).toEqual({ kind: "cursor", at: 0 });
    expect(answer("End", "field", 0, 3)).toEqual({ kind: "cursor", at: 2 });
  });

  test("Enter takes the cursor row and Escape closes back to the trigger (enter_adopts_the_cursor, escape_closes_and_keeps_the_value)", () => {
    expect(answer("Enter", "field", 1, 3)).toEqual({ kind: "take" });
    expect(answer("Escape", "field", 1, 3)).toEqual({ kind: "close", focus: "opener" });
    expect(claims(answer("Escape", "field", 1, 3))).toBe(true);
  });

  test("Tab closes and lets the focus leave, unclaimed (tab_lets_the_focus_leave)", () => {
    const tab = answer("Tab", "field", 1, 3);
    expect(tab).toEqual({ kind: "close", focus: "leave" });
    expect(claims(tab)).toBe(false);
  });

  test("the trigger answers Escape alone", () => {
    expect(answer("Escape", "trigger", 0, 3)).toEqual({ kind: "close", focus: "opener" });
    for (const key of [...MOVES, "Enter", "Tab", "a"]) expect(answer(key, "trigger", 0, 3)).toEqual({ kind: "pass" });
  });
});

describe("the value a look is given", () => {
  test("a key on the search box reaches the table, and a claimed key is taken from the platform", () => {
    const { look, record } = wired("open", 0);
    const down = pressing("ArrowDown");
    look.search.onkeydown(down);
    const tab = pressing("Tab");
    look.search.onkeydown(tab);
    expect(record.acted).toEqual([
      { kind: "cursor", at: 1 },
      { kind: "close", focus: "leave" },
    ]);
    expect([down.prevented(), tab.prevented()]).toEqual([true, false]);
  });

  test("a closed popup answers no key", () => {
    const { look, record } = wired("shut", 0);
    const escape = pressing("Escape");
    look.trigger.onkeydown(escape);
    look.search.onkeydown(pressing("ArrowDown"));
    expect(record.acted).toEqual([]);
    expect(escape.prevented()).toBe(false);
  });

  test("typing hands the box's text to the seat", () => {
    const { look, record } = wired("open", 2);
    look.search.oninput({ currentTarget: { value: "kimi" } });
    expect(record.typed).toEqual(["kimi"]);
  });

  test("aria-selected marks the value in force and the search box names the cursor", () => {
    const { look } = wired("open", 2, "glm-5");
    expect(look.options.map((each) => [each.wire["aria-selected"], each.chosen, each.cursor])).toEqual([
      [false, false, false],
      [true, true, false],
      [false, false, true],
    ]);
    expect(look.search["aria-activedescendant"]).toBe(rowId("c1", 2));
    expect(look.options.at(2)?.wire.id).toBe(rowId("c1", 2));
    expect(look.search["aria-controls"]).toBe(look.list.id);
    expect(look.search["aria-expanded"]).toBe(true);
    expect([look.face, look.unset]).toEqual(["glm-5", false]);
  });

  test("a closed popup names no cursor, takes no focus, and is not built before it first opens", () => {
    const shut = wired("shut", 1).look;
    expect([shut.search["aria-activedescendant"], shut.search["aria-expanded"], shut.layer?.inert]).toEqual([
      undefined,
      false,
      true,
    ]);
    expect(shut.options.some((each) => each.cursor)).toBe(false);
    expect(wired("unopened", 0).look.layer).toBeUndefined();
    expect([wired("unopened", 0).look.face, wired("unopened", 0).look.unset]).toEqual(["Search", true]);
  });

  test("a pointer over a row moves the cursor there and a click takes it", () => {
    const { look, record } = wired("open", 0);
    look.options.at(2)?.wire.onmouseenter();
    look.options.at(1)?.wire.onclick();
    expect([record.pointed, record.taken]).toEqual([[2], ["glm-5"]]);
  });
});
