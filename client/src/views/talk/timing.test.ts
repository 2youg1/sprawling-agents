// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, Seq, TimeMs, Tokens, type Call, type Timing, type Turn } from "../../wire";
import { kindOf } from "./call_kind";
import { rhythmOf } from "./rhythm";
import { callTime, lastedOf, runningWords, tookOf, tookWords, tpsOf, ttftOf, ttftTookOf } from "./timing";

function call(outcome: Call["outcome"], called: number, answered: number | null, timing: Timing = "measured"): Call {
  return {
    tool: "exec",
    subject: "just check",
    outcome,
    at: Seq.make(7),
    called: TimeMs.make(called),
    answered: answered === null ? null : TimeMs.make(answered),
    timing,
  };
}

function turn(t: number, first: number | null, timing: Timing = "measured", returned: number | null = null, output = 0): Turn {
  return {
    returned: returned === null ? null : TimeMs.make(returned),
    used: { input: Tokens.make(10), output: Tokens.make(output), cached: Tokens.make(0) },
    calls: [],
    notes: [],
    number: 1,
    opened: Seq.make(1),
    t: TimeMs.make(t),
    timing,
    first_at: first === null ? null : TimeMs.make(first),
  };
}

describe("a figure is a difference of two measured moments, or nothing", () => {
  test("a call that answered took the Ledger's milliseconds", () => {
    expect(lastedOf(call("answered", 1_000, 1_031))).toBe(31);
    expect(callTime(call("failed", 1_000, 4_412), 0)).toEqual({ kind: "landed", took: { unit: "ms", n: 3_412 } });
  });

  test("no figure for a span nobody measured, a missing answer, or moments out of order", () => {
    expect(lastedOf(call("answered", 1_000, 1_031, "unmeasured"))).toBeNull();
    expect(lastedOf(call("answered", 1_000, null))).toBeNull();
    expect(lastedOf(call("answered", 2_000, 1_000))).toBeNull();
    expect(callTime(call("answered", 1_000, 1_031, "unmeasured"), 0)).toEqual({ kind: "unmeasured" });
  });

  test("a running call counts from its own moment, and only past a second", () => {
    expect(callTime(call("waiting", 10_000, null), 10_900)).toEqual({ kind: "starting" });
    expect(callTime(call("waiting", 10_000, null), 14_700)).toEqual({ kind: "running", ms: 4_700 });
    expect(callTime(call("waiting", 10_000, null), 9_000)).toEqual({ kind: "starting" });
    expect(callTime(call("waiting", 10_000, null, "unmeasured"), 14_700)).toEqual({ kind: "starting" });
  });

  test("a call's own microseconds win over its moments; a line without them falls back to milliseconds", () => {
    expect(tookOf({ ...call("answered", 1_000, 1_000), took_us: 300 })).toEqual({ unit: "us", n: 300 });
    expect(tookOf(call("answered", 1_000, 1_012))).toEqual({ unit: "ms", n: 12 });
    expect(tookOf({ ...call("waiting", 1_000, null), took_us: 300 })).toBeNull();
    expect(ttftTookOf({ ...turn(5_000, 5_412), used: { input: Tokens.make(1), output: Tokens.make(1), cached: Tokens.make(0), first_us: 411_870 } })).toEqual({ unit: "us", n: 411_870 });
    expect(ttftTookOf(turn(5_000, 5_412))).toEqual({ unit: "ms", n: 412 });
  });

  test("time to first content is first_at minus the turn's own moment", () => {
    expect(ttftOf(turn(5_000, 5_412))).toBe(412);
    expect(ttftOf(turn(5_000, null))).toBeNull();
    expect(ttftOf(turn(5_000, 5_412, "unmeasured"))).toBeNull();
    expect(ttftOf(turn(5_000, 4_000))).toBeNull();
  });

  test("the output rate is the output count over first content to return, in seconds", () => {
    expect(tpsOf(turn(5_000, 5_400, "measured", 7_400, 120))).toBe(60);
  });

  test("no rate without both moments, a count, or a span that moves forward", () => {
    expect(tpsOf(turn(5_000, 5_400, "measured", null, 120))).toBeNull();
    expect(tpsOf(turn(5_000, null, "measured", 7_400, 120))).toBeNull();
    expect(tpsOf(turn(5_000, 5_400, "measured", 7_400, 0))).toBeNull();
    expect(tpsOf(turn(5_000, 5_400, "measured", 5_400, 120))).toBeNull();
    expect(tpsOf(turn(5_000, 5_400, "measured", 5_000, 120))).toBeNull();
    expect(tpsOf({ ...turn(5_000, 5_400, "measured", 7_400, 120), used: null })).toBeNull();
  });

  test("a landed figure is microseconds under 10 ms, milliseconds under a second, seconds above", () => {
    const us = (n: number) => tookWords({ unit: "us", n }, "en");
    const ms = (n: number) => tookWords({ unit: "ms", n }, "en");
    expect([us(300), us(9_999), us(12_000), us(3_412_000)]).toEqual(["300 µs", "9999 µs", "12 ms", "3.412 s"]);
    expect([ms(0), ms(31), ms(999), ms(3_412)]).toEqual(["0 ms", "31 ms", "999 ms", "3.412 s"]);
    expect([runningWords(4_749), runningWords(10_000)]).toEqual(["4.7 s", "10.0 s"]);
  });
});

describe("a tool line is named by what the tool was registered as", () => {
  const plain = call("answered", 0, 1);

  test("the drawing intent first, then the boundary, then the tool's own name", () => {
    expect(kindOf({ ...plain, render: "terminal", effect: "read" })).toEqual({ kind: "registered", word: "talk_kind_exec" });
    expect(kindOf({ ...plain, render: { diff: { locations: [] } } })).toEqual({ kind: "registered", word: "talk_kind_edit" });
    expect(kindOf({ ...plain, render: "generic", effect: { write: { domain: Address.make("lab") } } })).toEqual({
      kind: "registered",
      word: "talk_kind_write",
    });
    expect(kindOf({ ...plain, effect: { connector: { label: "github" } } })).toEqual({
      kind: "registered",
      word: "talk_kind_connector",
    });
    expect(kindOf({ ...plain, tool: "plan", effect: null, render: null })).toEqual({ kind: "unregistered", tool: "plan" });
  });
});

describe("the rhythm a reply arrived in", () => {
  test("nothing to draw from too few samples, no span, or no growth", () => {
    expect(rhythmOf([{ at: 0, length: 1 }, { at: 10, length: 5 }])).toBeNull();
    expect(rhythmOf([{ at: 5, length: 1 }, { at: 5, length: 2 }, { at: 5, length: 3 }])).toBeNull();
    expect(rhythmOf([{ at: 0, length: 4 }, { at: 5, length: 4 }, { at: 10, length: 4 }])).toBeNull();
  });

  test("each slice carries its share of the busiest, and a stall is an empty slice", () => {
    const shape = rhythmOf([
      { at: 0, length: 0 },
      { at: 10, length: 40 },
      { at: 20, length: 60 },
      { at: 200, length: 80 },
    ]);
    expect(shape?.length).toBe(20);
    expect(shape?.[1]).toBe(1);
    expect(shape?.[2]).toBe(0.5);
    expect(shape?.slice(3, 19).every((each) => each === 0)).toBe(true);
    expect(shape?.[19]).toBe(0.5);
  });
});
