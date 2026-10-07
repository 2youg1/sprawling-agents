// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The run page's sheet as decided, with no look: which facts stand on
// it for what the page knows, and where its two links lead
// (client/Spec.lean §4-42).

import { describe, expect, test } from "bun:test";

import { Address, Seq, TimeMs, Tokens, UsdMicros } from "../../wire";
import type { Turn } from "../../wire";
import { lookOf } from "./head";
import type { HeadProps } from "./head";

const TURN: Turn = {
  calls: [],
  notes: [],
  number: 1,
  opened: Seq.make(1),
  t: TimeMs.make(1_000),
  timing: "measured",
  model: "large-1",
  used: { input: Tokens.make(1_200), output: Tokens.make(80), cached: null },
  spent: UsdMicros.make(40_000),
};

const KNOWN: HeadProps = {
  turns: [TURN],
  doing: { kind: "frozen", completion: "done" },
  closing: { at: TimeMs.make(61_000), completion: "done" },
  room: Address.make("shop/checkout"),
  dispatchedBy: "person",
  from: 1_000,
  to: 61_000,
};

describe("the run page's sheet", () => {
  test("a run the page knows everything about stands every fact, the start two columns wide", () => {
    const look = lookOf(KNOWN, "en");
    expect(look.cells.map((cell) => [cell.key, cell.wide])).toEqual([
      ["outcome", false],
      ["took", true],
      ["tokens", false],
      ["spent", false],
      ["model", false],
      ["origin", false],
      ["dispatched", false],
    ]);
  });

  test("a fact the page does not know is left off rather than drawn empty", () => {
    const look = lookOf({ ...KNOWN, turns: [], room: null, dispatchedBy: null, from: null }, "en");
    expect(look.cells.map((cell) => cell.key)).toEqual(["outcome", "tokens", "spent"]);
  });

  test("where the run lives links its building and its room", () => {
    const origin = lookOf(KNOWN, "en").cells.find((cell) => cell.key === "origin");
    expect(origin?.lines).toEqual([
      {
        kind: "place",
        key: "shop/checkout",
        building: { text: "shop", wire: { href: "#/building/shop" } },
        room: { text: "checkout", wire: { href: "#/talk/shop/checkout" } },
      },
    ]);
  });

  test("the start is a moment a machine can read, and a cache nobody reported says so", () => {
    const look = lookOf(KNOWN, "en");
    const started = look.cells.find((cell) => cell.key === "took")?.lines[1];
    const cached = look.cells.find((cell) => cell.key === "tokens")?.lines[1];
    expect(started).toEqual({ kind: "time", key: "started", text: "started 1970-01-01 00:00:01.000Z", at: "1970-01-01T00:00:01.000Z" });
    expect(cached).toEqual({ kind: "text", key: "cached", rank: "more", face: "figure", text: "cache not reported" });
  });
});
