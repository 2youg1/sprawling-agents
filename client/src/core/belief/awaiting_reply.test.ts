// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "../belief";
import type { Doing } from "../doing";
import type { CityAnswer, EventKind, EventRecord, RunSummary } from "../../wire";
import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

const ONE = RunId.make("11111111-1111-4111-8111-111111111111");

function summary(run: RunId, at: number, kind: EventKind = "run_started"): RunSummary {
  return {
    run,
    addr: null,
    started: null,
    last_seq: Seq.make(at),
    last_kind: kind,
    frozen: false,
    who: "hall/mayor",
  };
}

function city(runs: readonly RunSummary[]): CityAnswer {
  return { active: runs.length, buildings: [], frozen: 0, halted: [], pursuits: [], runs };
}

function event(run: RunId, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
  return {
    run,
    seq: Seq.make(at),
    kind,
    t: TimeMs.make(at),
    who: "hall/mayor",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data,
  };
}

// One record of a run, which is how this page first hears of a run the
// city has not listed to it yet.
function started(run: RunId, at: number): EventRecord {
  return event(run, at, "run_started", { task: "write the report" });
}

// client/Spec.lean D88: the stream's route and the answer's route to a
// run stopped at a synchronous `send`.
describe("a run waiting for a reply", () => {
  const WAIT = { on: Address.make("lab/west"), until: TimeMs.make(90_000) };
  const AWAITING: Doing = { kind: "awaiting_reply", wait: WAIT };

  function waiting(): ReturnType<typeof createBelief> {
    const store = createBelief(() => 0);
    store.apply(started(ONE, 1));
    expect(store.apply(event(ONE, 2, "signal_wait_started", { on: "lab/west", signal: "r-s1", deadline_ms: 90_000 }))).toBeNull();
    return store;
  }

  test("the record that starts the wait names the room and the deadline", () => {
    expect(get(waiting().belief).runs[ONE]?.doing).toEqual(AWAITING);
  });

  test("each ending leaves the wait", () => {
    const ends = [
      [{ end: "reply", reply: "w-s1" }, { kind: "thinking" }],
      [{ end: "timeout" }, { kind: "thinking" }],
      [{ end: "left" }, { kind: "unknown" }],
    ] as const;
    for (const [by, after] of ends) {
      const store = waiting();
      expect(store.apply(event(ONE, 3, "signal_wait_ended", { signal: "r-s1", by }))).toBeNull();
      expect(get(store.belief).runs[ONE]?.doing).toEqual(after);
    }
  });

  // A word this build was not taught is answered, and the run stays
  // where the page last knew it.
  test("an ending this build cannot read leaves the run waiting and names the field", () => {
    const store = waiting();
    expect(store.apply(event(ONE, 3, "signal_wait_ended", { signal: "r-s1", by: { end: "shrugged" } }))).toBe("signal_wait_ended.by.end");
    expect(get(store.belief).runs[ONE]?.doing).toEqual(AWAITING);
  });

  test("a start this build cannot read names the field and states no wait", () => {
    const store = createBelief(() => 0);
    store.apply(started(ONE, 1));
    expect(store.apply(event(ONE, 2, "signal_wait_started", { on: "lab/west", signal: "r-s1" }))).toBe("signal_wait_started.deadline_ms");
    expect(get(store.belief).runs[ONE]?.doing).toEqual({ kind: "thinking" });
  });

  test("an answer that names a wait states it, and one that no longer does ends it", () => {
    const store = createBelief(() => 0);
    store.adoptCity(city([{ ...summary(ONE, 5, "signal_wait_started"), waiting: WAIT }]));
    expect(get(store.belief).runs[ONE]?.doing).toEqual(AWAITING);

    store.adoptCity(city([summary(ONE, 6, "signal_wait_ended")]));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({ kind: "unknown" });
  });
});
