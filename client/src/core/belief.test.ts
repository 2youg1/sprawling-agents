// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { createBelief } from "./belief";
import { sendingInto, type Doing } from "./doing";
import type { CityAnswer, EventKind, EventRecord, RunSummary } from "../wire";
import { Address, B3Hash, CITY_RUN, RunId, Seq, TimeMs } from "../wire";

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

  // A phase nobody stated is not a promise that the words land at a
  // boundary, and the queue is the spelling that promises least.
  test("a run whose phase this page was never told queues", () => {
    expect(sendingInto({ kind: "unknown" })).toBe("queued");
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
const NIL = RunId.make(CITY_RUN);

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

// One record of the city itself. A halt is about the whole city or one
// of its buildings, so it carries the nil run id every city-level record
// carries.
function halt(at: number, data: Record<string, unknown>): EventRecord {
  return event(NIL, at, "city_halted", data);
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

describe("the phase a run is in", () => {
  // The defect: a run adopted from an answer was drawn as `thinking`,
  // while the row's own `last_kind` said it was stopped at an approval
  // - the page told a person nothing waited for them while the city was
  // waiting for them.
  test("an answer's last kind settles the phase it can state", () => {
    const store = createBelief();
    store.adoptCity(city([summary(ONE, 7, "approval_requested")]));
    expect(store.belief.runs[ONE]?.doing).toEqual({ kind: "waiting" });
  });

  // A kind that states no phase does not overwrite one the page read
  // off the stream: the answer is newer, and it still says nothing here.
  test("a kind that states no phase leaves the page's own reading alone", () => {
    const store = createBelief();
    store.apply(event(ONE, 4, "approval_requested", {}));
    expect(store.belief.runs[ONE]?.doing).toEqual({ kind: "waiting" });

    store.adoptCity(city([summary(ONE, 6, "steer_received")]));
    expect(store.belief.runs[ONE]?.doing).toEqual({ kind: "waiting" });
  });

  test("a run this page never saw is unknown rather than thinking", () => {
    const store = createBelief();
    store.adoptCity(city([summary(ONE, 7, "steer_received")]));
    expect(store.belief.runs[ONE]?.doing).toEqual({ kind: "unknown" });
  });

  // An answer carries no payload, so the phase it states is the whole of
  // what it can say about a call: the call is named, the tool is not.
  test("a run whose last kind is a tool call is calling with no tool named", () => {
    const store = createBelief();
    store.adoptCity(city([summary(ONE, 7, "tool_called")]));
    expect(store.belief.runs[ONE]?.doing).toEqual({
      kind: "calling",
      tool: null,
      subject: null,
    });
  });
});

describe("the scopes a person shut", () => {
  // The defect: an answer replaced the whole list while a `city_halted`
  // record changed one scope, and nothing compared them by position - so
  // a gap folded back in after the answer undid it.
  test("a halt older than the answer that answered it changes nothing", () => {
    const store = createBelief();
    store.apply(halt(10, { scope: "building:hall", state: "halted" }));
    expect(store.belief.halted).toEqual([{ building: Address.make("hall") }]);

    // An answer folded at 20 says the building takes work again.
    store.adoptCity(city([summary(ONE, 20)]));
    expect(store.belief.halted).toEqual([]);

    // The record it already folded, delivered again as a gap page.
    store.apply(halt(10, { scope: "building:hall", state: "halted" }));
    expect(store.belief.halted).toEqual([]);

    // A halt newer than the answer still lands.
    store.apply(halt(21, { scope: "city", state: "halted" }));
    expect(store.belief.halted).toEqual(["city"]);
  });

  // An answer whose runs are all behind the list cannot take the page
  // back over what it has already folded.
  test("an answer older than the list does not release a scope", () => {
    const store = createBelief();
    store.apply(halt(10, { scope: "city", state: "halted" }));
    store.adoptCity(city([summary(ONE, 5)]));
    expect(store.belief.halted).toEqual(["city"]);
  });
});

describe("reading one record", () => {
  // The defect: every payload key was read through a helper that
  // answered null for a value present in another shape, so a key this
  // build had mis-spelled or mis-typed read as "this did not happen".
  test("a payload this build cannot read answers the field", () => {
    const store = createBelief();
    const bad = store.apply(event(ONE, 3, "tool_called", { name: 7, subject: "a file" }));
    expect(bad).toBe("tool_called.name");
    // The rest of the record is still folded: the position and the one
    // field that did read are facts, and only the call's name is lost.
    expect(store.belief.runs[ONE]?.lastSeq).toBe(Seq.make(3));
    expect(store.belief.runs[ONE]?.doing).toEqual({
      kind: "calling",
      tool: null,
      subject: "a file",
    });
  });

  // A record whose two words decide between stopping and starting is
  // not read as a release when one of them is a word this build does not
  // know.
  test("a halt word this build does not know changes nothing", () => {
    const store = createBelief();
    const bad = store.apply(halt(4, { scope: "city", state: "frozen" }));
    expect(bad).toBe("city_halted.state");
    expect(store.belief.halted).toEqual([]);
  });

  // The two absences the structs state: `ToolCalled::subject` is an
  // `Option` and `RunStarted::task` carries `#[serde(default)]`.
  test("an absent optional key is a state and not a failure", () => {
    const store = createBelief();
    expect(store.apply(event(ONE, 3, "tool_called", { name: "exec" }))).toBeNull();
    expect(store.belief.runs[ONE]?.doing).toEqual({
      kind: "calling",
      tool: "exec",
      subject: null,
    });
  });

  test("an absent task reads the empty string the struct defaults to", () => {
    const store = createBelief();
    expect(store.apply(event(ONE, 3, "run_started", {}))).toBeNull();
    expect(store.belief.runs[ONE]?.task).toBe("");
  });
});

describe("text for a run this page has not met", () => {
  // A page that joins in the middle of a call hears the model before it
  // hears the run. Dropping those words lost the beginning of the
  // sentence a person was reading; the run's own records fill in the
  // address, the task and the phase as they arrive.
  test("a delta that outran its run is held rather than dropped", () => {
    const store = createBelief();
    store.say({ run: ONE, increment: { said: "half a sentence" } });
    expect(store.belief.runs[ONE]?.saying).toBe("half a sentence");
    expect(store.belief.runs[ONE]?.local).toBe(true);

    store.apply(started(ONE, 4));
    expect(store.belief.runs[ONE]?.task).toBe("write the report");
    expect(store.belief.runs[ONE]?.saying).toBe("half a sentence");
  });
});
