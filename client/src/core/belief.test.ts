// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { createBelief, sendingInto, type Doing } from "./belief";
import type { CityAnswer, EventRecord, RunSummary } from "../wire";
import { B3Hash, RunId, Seq, TimeMs } from "../wire";

describe("sending", () => {
  // The defect this pins is one of wording, and it is the one a
  // streaming page invites: text arriving letter by letter reads as a
  // conversation, and a person types into it expecting to be heard. A
  // steer is consumed at a phase boundary, so while a tool call is out
  // the words wait - and the composer has to say so.
  test("a run that is blocked queues the words instead of saying them", () => {
    const calling: Doing = { kind: "calling", tool: "exec", subject: "just check" };
    expect(sendingInto(calling)).toBe("queued");
    expect(sendingInto({ kind: "waiting" })).toBe("queued");
  });

  test("a run between calls hears a steer at the next boundary", () => {
    expect(sendingInto({ kind: "thinking" })).toBe("steer");
  });

  // Nothing in flight and a run that has ended are the same fact for a
  // person writing: the next message opens work rather than interrupting
  // it. `undefined` is "this room has no live run", not "unknown".
  test("no live run and a frozen one both open new work", () => {
    expect(sendingInto(undefined)).toBe("dispatch");
    expect(sendingInto({ kind: "frozen", completion: "done" })).toBe("dispatch");
    expect(sendingInto({ kind: "frozen", completion: null })).toBe("dispatch");
  });
});

const ONE = RunId.make("11111111-1111-4111-8111-111111111111");
const TWO = RunId.make("22222222-2222-4222-8222-222222222222");

function summary(run: RunId, at: number): RunSummary {
  return {
    run,
    addr: null,
    started: null,
    last_seq: Seq.make(at),
    last_kind: "run_started",
    frozen: false,
    who: "hall/mayor",
  };
}

function city(runs: readonly RunSummary[]): CityAnswer {
  return { active: runs.length, buildings: [], frozen: 0, halted: [], pursuits: [], runs };
}

// One `run_started`, which is how this page first hears of a run the
// city has not listed to it yet.
function started(run: RunId, at: number): EventRecord {
  return {
    run,
    seq: Seq.make(at),
    kind: "run_started",
    t: TimeMs.make(at),
    who: "hall/mayor",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data: { task: "write the report" },
  };
}

describe("the runs a page believes in", () => {
  // The defect: `adoptCity` only ever added. A city that restarted, or
  // a `run_frozen` that landed while the socket was down, left a run
  // that says `thinking` for the life of the tab - counting in the
  // skyline, the metrics and the tab's title while nothing runs.
  test("a run the answer no longer mentions is dropped", () => {
    const store = createBelief();
    store.adoptCity(city([summary(ONE, 7), summary(TWO, 9)]));
    expect(Object.keys(store.belief.runs).sort()).toEqual([ONE, TWO].sort());

    store.adoptCity(city([summary(TWO, 11)]));
    expect(Object.keys(store.belief.runs)).toEqual([TWO]);
    expect(store.belief.runs[TWO]?.lastSeq).toBe(Seq.make(11));
  });

  // The exception, and the reason a boolean has to carry it: a run
  // dispatched a moment ago is missing from an answer folded before it
  // started, which is not the city saying the run is over. `CityAnswer`
  // states no ledger position, so only the page can tell the two apart.
  test("a run the stream introduced survives an answer that predates it", () => {
    const store = createBelief();
    store.apply(started(ONE, 4));
    expect(store.belief.runs[ONE]?.task).toBe("write the report");

    store.adoptCity(city([summary(TWO, 2)]));
    expect(Object.keys(store.belief.runs).sort()).toEqual([ONE, TWO].sort());

    // And once an answer has named it, the city owns when it goes.
    store.adoptCity(city([summary(ONE, 4), summary(TWO, 2)]));
    store.adoptCity(city([summary(TWO, 2)]));
    expect(Object.keys(store.belief.runs)).toEqual([TWO]);
  });

  // The exemption is good for one answer, which is what keeps it from
  // becoming the ghost it was written to prevent: a city that restarts
  // and no longer knows the run gets to say so with its second answer.
  test("a run no second answer lists is dropped even if the stream began it", () => {
    const store = createBelief();
    store.apply(started(ONE, 4));

    store.adoptCity(city([summary(TWO, 2)]));
    store.adoptCity(city([summary(TWO, 3)]));
    expect(Object.keys(store.belief.runs)).toEqual([TWO]);
  });

  // A run the page is ahead of keeps its own reading: the answer is a
  // position, and the page holds more than a position.
  test("an answer older than the stream does not roll a run back", () => {
    const store = createBelief();
    store.apply(started(ONE, 12));
    store.adoptCity(city([summary(ONE, 5)]));
    expect(store.belief.runs[ONE]?.lastSeq).toBe(Seq.make(12));
    expect(store.belief.runs[ONE]?.task).toBe("write the report");
  });
});
