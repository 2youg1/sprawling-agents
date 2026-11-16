// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "./belief";
import type {
  AxError,
  CityAnswer,
  EventKind,
  EventRecord,
  Mode,
  RunPolicy,
  RunSummary,
} from "../wire";
import { Address, B3Hash, CITY_RUN, RunId, Seq, TimeMs } from "../wire";

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

function event(
  run: RunId,
  at: number,
  kind: EventKind,
  data: Record<string, unknown>,
): EventRecord {
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

function started(run: RunId, at: number): EventRecord {
  return event(run, at, "run_started", { task: "write the report" });
}

function halt(at: number, data: Record<string, unknown>): EventRecord {
  return event(NIL, at, "city_halted", data);
}

describe("the runs a page believes in", () => {
  test("a run the answer no longer mentions is dropped", () => {
    const store = createBelief(() => 0);
    store.adoptCity(city([summary(ONE, 7), summary(TWO, 9)]));
    expect(Object.keys(get(store.belief).runs).sort()).toEqual([ONE, TWO].sort());

    store.adoptCity(city([summary(TWO, 11)]));
    expect(Object.keys(get(store.belief).runs)).toEqual([TWO]);
    expect(get(store.belief).runs[TWO]?.lastSeq).toBe(Seq.make(11));
  });

  test("a run the stream introduced survives an answer that predates it", () => {
    const store = createBelief(() => 0);
    store.apply(started(ONE, 4));
    expect(get(store.belief).runs[ONE]?.task).toBe("write the report");

    store.adoptCity(city([summary(TWO, 2)]));
    expect(Object.keys(get(store.belief).runs).sort()).toEqual([ONE, TWO].sort());

    store.adoptCity(city([summary(ONE, 4), summary(TWO, 2)]));
    store.adoptCity(city([summary(TWO, 2)]));
    expect(Object.keys(get(store.belief).runs)).toEqual([TWO]);
  });

  test("a run no second answer lists is dropped even if the stream began it", () => {
    const store = createBelief(() => 0);
    store.apply(started(ONE, 4));

    store.adoptCity(city([summary(TWO, 2)]));
    store.adoptCity(city([summary(TWO, 3)]));
    expect(Object.keys(get(store.belief).runs)).toEqual([TWO]);
  });

  test("an answer older than the stream does not roll a run back", () => {
    const store = createBelief(() => 0);
    store.apply(started(ONE, 12));
    store.adoptCity(city([summary(ONE, 5)]));
    expect(get(store.belief).runs[ONE]?.lastSeq).toBe(Seq.make(12));
    expect(get(store.belief).runs[ONE]?.task).toBe("write the report");
  });

  test("a run met only in an answer keeps the task and goal the answer carries", () => {
    const store = createBelief(() => 0);
    store.adoptCity(city([{ ...summary(ONE, 5), task: "draft the plan", goal: "a plan in hall" }]));
    const held = get(store.belief).runs[ONE];
    expect([held?.task, held?.goal]).toEqual(["draft the plan", "a plan in hall"]);
  });
});

describe("the phase a run is in", () => {
  test("an answer's last kind settles the phase it can state", () => {
    const store = createBelief(() => 0);
    store.adoptCity(city([summary(ONE, 7, "approval_requested")]));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({ kind: "waiting" });
  });

  test("a kind that states no phase leaves the page's own reading alone", () => {
    const store = createBelief(() => 0);
    store.apply(event(ONE, 4, "approval_requested", {}));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({ kind: "waiting" });

    store.adoptCity(city([summary(ONE, 6, "steer_received")]));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({ kind: "waiting" });
  });

  test("a run this page never saw is unknown rather than thinking", () => {
    const store = createBelief(() => 0);
    store.adoptCity(city([summary(ONE, 7, "steer_received")]));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({ kind: "unknown" });
  });

  test("a run whose last kind is a tool call is calling with no tool named", () => {
    const store = createBelief(() => 0);
    store.adoptCity(city([summary(ONE, 7, "tool_called")]));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({
      kind: "calling",
      tool: null,
      subject: null,
    });
  });
});

describe("the scopes a person shut", () => {
  test("a halt older than the answer that answered it changes nothing", () => {
    const store = createBelief(() => 0);
    store.apply(halt(10, { scope: "building:hall", state: "halted" }));
    expect(get(store.belief).halted).toEqual([{ building: Address.make("hall") }]);

    store.adoptCity(city([summary(ONE, 20)]));
    expect(get(store.belief).halted).toEqual([]);

    store.apply(halt(10, { scope: "building:hall", state: "halted" }));
    expect(get(store.belief).halted).toEqual([]);

    store.apply(halt(21, { scope: "city", state: "halted" }));
    expect(get(store.belief).halted).toEqual(["city"]);
  });

  test("an answer older than the list does not release a scope", () => {
    const store = createBelief(() => 0);
    store.apply(halt(10, { scope: "city", state: "halted" }));
    store.adoptCity(city([summary(ONE, 5)]));
    expect(get(store.belief).halted).toEqual(["city"]);
  });
});

describe("reading one record", () => {
  test("a payload this build cannot read answers the field", () => {
    const store = createBelief(() => 0);
    const bad = store.apply(event(ONE, 3, "tool_called", { name: 7, subject: "a file" }));
    expect(bad).toBe("tool_called.name");
    expect(get(store.belief).runs[ONE]?.lastSeq).toBe(Seq.make(3));
    expect(get(store.belief).runs[ONE]?.doing).toEqual({
      kind: "calling",
      tool: null,
      subject: "a file",
    });
  });

  test("a halt word this build does not know changes nothing", () => {
    const store = createBelief(() => 0);
    const bad = store.apply(halt(4, { scope: "city", state: "frozen" }));
    expect(bad).toBe("city_halted.state");
    expect(get(store.belief).halted).toEqual([]);
  });

  test("an absent optional key is a state and not a failure", () => {
    const store = createBelief(() => 0);
    expect(store.apply(event(ONE, 3, "tool_called", { name: "exec" }))).toBeNull();
    expect(get(store.belief).runs[ONE]?.doing).toEqual({
      kind: "calling",
      tool: "exec",
      subject: null,
    });
  });

  test("an absent task reads the empty string the struct defaults to", () => {
    const store = createBelief(() => 0);
    expect(store.apply(event(ONE, 3, "run_started", {}))).toBeNull();
    expect(get(store.belief).runs[ONE]?.task).toBe("");
  });
});

describe("text for a run this page has not met", () => {
  test("a delta that outran its run is held rather than dropped", () => {
    const store = createBelief(() => 0);
    store.say({ run: ONE, increment: { said: "half a sentence" } });
    expect(get(store.belief).runs[ONE]?.saying).toBe("half a sentence");
    expect(get(store.belief).runs[ONE]?.local).toBe(true);

    store.apply(started(ONE, 4));
    expect(get(store.belief).runs[ONE]?.task).toBe("write the report");
    expect(get(store.belief).runs[ONE]?.saying).toBe("half a sentence");
  });
});

describe("one refusal, however often it arrived", () => {
  const BUSY: AxError = {
    code: "E_BUSY",
    action: "dispatch the task",
    subject: "hall/mayor",
    nearby: [],
    retry: "no",
    recovery: "",
  };

  test("the same code and subject are one notice with a count", () => {
    let at = 1_000;
    const store = createBelief(() => at);
    store.refused(BUSY);
    at = 5_000;
    store.refused({ ...BUSY, recovery: "wait for the run to end" });

    expect(get(store.belief).notices).toEqual([
      {
        error: { ...BUSY, recovery: "wait for the run to end" },
        seen: false,
        at: TimeMs.make(1_000),
        key: "E_BUSY\u0000hall/mayor",
        count: 2,
        about: Address.make("hall/mayor"),
      },
    ]);
  });

  test("a repeat clears the read mark, and another subject is another notice", () => {
    const store = createBelief(() => 0);
    store.refused(BUSY);
    store.noticesSeen();
    store.refused(BUSY);
    store.refused({ ...BUSY, subject: "hall/clerk" });

    expect(get(store.belief).notices).toEqual([
      {
        error: BUSY,
        seen: false,
        at: TimeMs.make(0),
        key: "E_BUSY\u0000hall/mayor",
        count: 2,
        about: Address.make("hall/mayor"),
      },
      {
        error: { ...BUSY, subject: "hall/clerk" },
        seen: false,
        at: TimeMs.make(0),
        key: "E_BUSY\u0000hall/clerk",
        count: 1,
        about: Address.make("hall/clerk"),
      },
    ]);
  });

  test("the window drops the oldest refusal, not the newest", () => {
    const store = createBelief(() => 0);
    for (let n = 0; n < 51; n += 1) {
      store.refused({ ...BUSY, subject: `hall/room${String(n)}` });
    }
    const notices = get(store.belief).notices;
    expect(notices).toHaveLength(50);
    expect(notices[0]?.error.subject).toBe("hall/room1");
  });

  test("the subject is read as a run, a room, or nothing", () => {
    const store = createBelief(() => 0);
    store.refused({ ...BUSY, subject: ONE });
    store.refused({ ...BUSY, subject: "wire: ours 3, theirs 2" });
    const notices = get(store.belief).notices;
    expect(notices[0]?.about).toBe(ONE);
    expect(notices[1]?.about).toBeNull();
  });
});

describe("batch", () => {
  test("a fold that fails inside a batch leaves later writes published", () => {
    const store = createBelief(() => 0);
    expect(() => {
      store.batch(() => {
        JSON.parse("{");
      });
    }).toThrow();
    store.named("city");

    expect(get(store.belief).city).toBe("city");
  });
});

describe("the model a session answers with", () => {
  test("the model a run's call went to is the run's model", () => {
    const store = createBelief(() => 0);
    store.apply(started(ONE, 2));
    expect(get(store.belief).runs[ONE]?.model).toBeNull();
    expect(
      store.apply(event(ONE, 3, "model_called", { segments: [], model: "fake-small" })),
    ).toBeNull();
    expect(get(store.belief).runs[ONE]?.model).toBe("fake-small");
  });
});

describe("the run policy a room holds", () => {
  test("the newest readable change holds per room, in any arrival order", () => {
    const store = createBelief(() => 0);
    const room = Address.make("lab/policy");
    const policy = (mode: Mode): RunPolicy => ({
      admit: "standing",
      landing: "ordinary",
      mode,
      write: "create",
    });
    const change = (at: number, data: Record<string, unknown>): EventRecord => ({
      ...event(ONE, at, "run_policy_changed", data),
      addr: room,
    });
    store.apply(change(9, { policy: policy("work"), by: "person" }));
    store.apply(change(5, { policy: policy("chat"), by: "person" }));
    store.apply(change(12, { policy: { mode: 3 }, by: "person" }));
    expect(get(store.belief).policies).toEqual({
      [room]: { addr: room, seq: Seq.make(9), at: TimeMs.make(9), policy: policy("work") },
    });
  });
});
