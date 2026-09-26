// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "../belief";
import { newestWorking } from "./live";
import { heldIn, heldWithin } from "./rooms";
import type { RunBelief } from "./shape";
import type { CityAnswer, EventKind, EventRecord, RunSummary } from "../../wire";
import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

const RUNS = 10_000;
// The runs still working in that city: the concurrency a city drives,
// not the history it has kept.
const WORKING = 4;
const RECORDS = 2_000;
// What one record folded and one read of the working runs may spend in
// a city of `RUNS` runs. A read that walks the `$state` table is linear
// in the city, about 18 ms per record at this size; the fold itself
// costs tens of microseconds of proxy traffic whatever the city holds,
// so the budget is set well above that and far below the walk.
const BUDGET_US = 500;

function runId(index: number): RunId {
  return RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`);
}

function listed(index: number, addr: Address | null = null): RunSummary {
  return {
    run: runId(index),
    addr,
    started: TimeMs.make(index),
    last_seq: Seq.make(1),
    last_kind: "run_started",
    frozen: index >= WORKING,
    who: "hall/mayor",
  };
}

function record(index: number, at: number, kind: EventKind): EventRecord {
  return {
    run: runId(index),
    seq: Seq.make(at),
    kind,
    t: TimeMs.make(at),
    who: "hall/mayor",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data: {},
  };
}

test("reading the working runs costs the working runs, not the city", () => {
  const store = createBelief(() => 0);
  const city: CityAnswer = {
    active: WORKING,
    buildings: [],
    frozen: RUNS - WORKING,
    halted: [],
    pursuits: [],
    runs: Array.from({ length: RUNS }, (_, index) => listed(index)),
  };
  store.adoptCity(city);
  // The first half warms the folds up, so the reading is the steady cost
  // of a record rather than the engine compiling the path.
  const records = Array.from({ length: 2 * RECORDS }, (_, each) =>
    record(each % WORKING, each + 2, "tool_result"),
  );
  let working = 0;
  let start = 0;
  for (const [at, each] of records.entries()) {
    if (at === RECORDS) start = performance.now();
    store.apply(each);
    working += get(store.belief).live.length;
  }
  const perRecordUs = ((performance.now() - start) * 1000) / RECORDS;
  store.apply(record(0, 2 * RECORDS + 2, "run_frozen"));
  expect({
    working,
    after: get(store.belief).live.map((run) => run.run),
  }).toEqual({ working: 2 * WORKING * RECORDS, after: [runId(1), runId(2), runId(3)] });
  expect(perRecordUs).toBeLessThanOrEqual(BUDGET_US);
});

test("the newest working run of a room costs the working runs, not the city", () => {
  const store = createBelief(() => 0);
  // Every run of the city stood in the one room, so a reader that walks
  // the table to find the room's runs walks the whole city.
  const room = Address.make("hall/mayor");
  store.adoptCity({
    active: WORKING,
    buildings: [],
    frozen: RUNS - WORKING,
    halted: [],
    pursuits: [],
    runs: Array.from({ length: RUNS }, (_, index) => listed(RUNS - 1 - index, room)),
  });
  const records = Array.from({ length: 2 * RECORDS }, (_, each) =>
    record(each % WORKING, each + 2, "tool_result"),
  );
  let start = 0;
  let newest: RunBelief | undefined;
  for (const [at, each] of records.entries()) {
    if (at === RECORDS) start = performance.now();
    store.apply(each);
    newest = newestWorking(get(store.belief), room);
  }
  const perRecordUs = ((performance.now() - start) * 1000) / RECORDS;
  store.apply(record(WORKING - 1, 2 * RECORDS + 2, "run_frozen"));
  expect({
    steady: newest?.run,
    after: newestWorking(get(store.belief), room)?.run,
    elsewhere: newestWorking(get(store.belief), "hall"),
  }).toEqual({ steady: runId(WORKING - 1), after: runId(WORKING - 2), elsewhere: undefined });
  expect(perRecordUs).toBeLessThanOrEqual(BUDGET_US);
});

test("the runs of a room cost the room, not the city", () => {
  const store = createBelief(() => 0);
  // A city of `RUNS` runs spread over a hundred rooms under the hall.
  const rooms = 100;
  const roomOf = (index: number): Address => Address.make(`hall/r${String(index % rooms)}`);
  store.adoptCity({
    active: WORKING,
    buildings: [],
    frozen: RUNS - WORKING,
    halted: [],
    pursuits: [],
    runs: Array.from({ length: RUNS }, (_, index) => listed(RUNS - 1 - index, roomOf(RUNS - 1 - index))),
  });
  const reads = 200;
  const records = Array.from({ length: 2 * reads }, (_, each) =>
    record(each % WORKING, each + 2, "tool_result"),
  );
  let start = 0;
  let held = 0;
  for (const [at, each] of records.entries()) {
    if (at === reads) start = performance.now();
    store.apply(each);
    held += heldIn(get(store.belief), roomOf(0)).length;
  }
  const perRecordUs = ((performance.now() - start) * 1000) / reads;
  const belief = get(store.belief);
  expect({
    held,
    first: heldIn(belief, roomOf(1)).slice(0, 3).map((run) => run.run),
    within: heldWithin(belief, "hall").length,
    none: heldIn(belief, "hall").length,
  }).toEqual({
    held: 2 * reads * (RUNS / rooms),
    first: [runId(1), runId(1 + rooms), runId(1 + 2 * rooms)],
    within: RUNS,
    none: 0,
  });
  expect(perRecordUs).toBeLessThanOrEqual(BUDGET_US);
});

test("the cancelled count costs one run a record, not the city", () => {
  const store = createBelief(() => 0);
  store.adoptCity({
    active: WORKING,
    buildings: [],
    frozen: RUNS - WORKING,
    halted: [],
    pursuits: [],
    runs: Array.from({ length: RUNS }, (_, index) => listed(index)),
  });
  const reads = 200;
  let start = 0;
  let during = 0;
  for (let at = 0; at < 2 * reads; at += 1) {
    if (at === reads) start = performance.now();
    store.apply(record(at % WORKING, at + 2, "tool_result"));
    during += get(store.belief).cancelled;
  }
  const perRecordUs = ((performance.now() - start) * 1000) / reads;
  store.apply({ ...record(0, 2 * reads + 2, "run_frozen"), data: { completion: "cancelled" } });
  const once = get(store.belief).cancelled;
  store.apply({ ...record(0, 2 * reads + 3, "run_frozen"), data: { completion: "cancelled" } });
  expect({ during, once, again: get(store.belief).cancelled }).toEqual({ during: 0, once: 1, again: 1 });
  expect(perRecordUs).toBeLessThanOrEqual(BUDGET_US);
});
