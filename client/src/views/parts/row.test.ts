// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The walk a list of rows takes, replayed against the properties of
// `client/spec/Views/Parts/Row.lean`, and the two bags a row's look is
// handed. Where a row landed on the page is measured by
// `cargo xtask render` on the gallery fixtures.

import { describe, expect, test } from "bun:test";

import { landing, lookOf, taken, type Step, type Target } from "./row";

const STEPS: readonly Step[] = ["next", "previous", "first", "last"];
const KEYS = ["ArrowDown", "ArrowUp", "Home", "End", "Tab", "Enter", "j"] as const;
const FIELDS: readonly Target[] = ["text", "select", "editable"];

// Every row offers a control, named by its position.
function rows(total: number): readonly number[] {
  return Array.from({ length: total }, (_unused, at) => at);
}

describe("which keys the list takes", () => {
  test("the four walking keys on a row, and nothing else", () => {
    expect(KEYS.map((key) => taken("row", key))).toEqual(["next", "previous", "first", "last", null, null, null]);
  });

  test("a field keeps every key", () => {
    for (const target of FIELDS) {
      expect(KEYS.map((key) => taken(target, key))).toEqual(KEYS.map(() => null));
    }
  });
});

describe("where a step lands", () => {
  test("a step stays inside the list from every row", () => {
    for (const total of [1, 2, 7]) {
      for (const from of rows(total)) {
        for (const step of STEPS) {
          const at = landing(rows(total), step, from);
          expect(at === undefined || (at >= 0 && at < total)).toBe(true);
        }
      }
    }
  });

  test("the ends hold rather than wrap", () => {
    expect(landing(rows(1000), "next", 999)).toBeUndefined();
    expect(landing(rows(1000), "previous", 0)).toBeUndefined();
  });

  test("Home and End reach the ends from anywhere", () => {
    expect([landing(rows(5), "first", 3), landing(rows(5), "last", 1)]).toEqual([0, 4]);
  });

  test("a row offering no control is stepped over in either direction", () => {
    const reach = ["a", undefined, "c", undefined];
    expect([
      landing(reach, "next", 0),
      landing(reach, "previous", 2),
      landing(reach, "next", 2),
      landing(reach, "last", 0),
    ]).toEqual(["c", "a", undefined, "c"]);
  });
});

describe("what a row's look is handed", () => {
  test("a plain row leads nowhere and is not chosen", () => {
    expect(lookOf({ primary: "zenmux", secondary: "two models" })).toEqual({
      item: {},
      chosen: false,
      primary: "zenmux",
      secondary: "two models",
      status: undefined,
      actions: undefined,
      open: undefined,
    });
  });

  test("a row that leads somewhere hands its press to a button", () => {
    let opened = 0;
    const look = lookOf({
      primary: "zenmux",
      onOpen: () => {
        opened += 1;
      },
    });
    look.open?.onclick();
    expect([look.open?.type, opened]).toEqual(["button", 1]);
  });

  test("the chosen row says so to a screen reader as well as with the bar", () => {
    const look = lookOf({ primary: "zenmux", chosen: true });
    expect([look.item, look.chosen]).toEqual([{ "aria-current": "true" }, true]);
  });
});
