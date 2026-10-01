// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "./belief";
import { runInFront } from "./in_front";
import type { RunSummary } from "../wire";
import { Address, RunId, Seq, TimeMs } from "../wire";

const SHOP = Address.make("shop/front");
const YARD = Address.make("yard/gate");
const QUIET = Address.make("hall/quiet");

function runId(tail: number): RunId {
  return RunId.make(`00000000-0000-4000-8000-${tail.toString(16).padStart(12, "0")}`);
}

function listed(run: RunId, addr: Address, started: number, frozen: boolean): RunSummary {
  return {
    run,
    addr,
    started: TimeMs.make(started),
    last_seq: Seq.make(1),
    last_kind: "run_started",
    frozen,
    who: "hall/mayor",
  };
}

// The regression client-SPEC 12-16 names: a stop pressed in one room
// reached the newest run anywhere in the city, and a stop pressed where
// no run is shown reached one all the same.
test("the run in front of the person is the one the page shows, never the newest anywhere", () => {
  const shop = runId(1);
  const yard = runId(2);
  const done = runId(3);
  const store = createBelief(() => 0);
  store.adoptCity({
    active: 2,
    buildings: [],
    frozen: 1,
    halted: [],
    pursuits: [],
    runs: [listed(done, SHOP, 0, true), listed(shop, SHOP, 1, false), listed(yard, YARD, 2, false)],
  });
  const held = get(store.belief);
  expect({
    talk: runInFront(held, { kind: "talk", address: SHOP })?.run,
    quietRoom: runInFront(held, { kind: "talk", address: QUIET })?.run,
    runPage: runInFront(held, { kind: "run", run: shop })?.run,
    finishedRunPage: runInFront(held, { kind: "run", run: done })?.run,
    city: runInFront(held, { kind: "city" })?.run,
    building: runInFront(held, { kind: "building", address: SHOP })?.run,
  }).toEqual({
    talk: shop,
    quietRoom: undefined,
    runPage: shop,
    finishedRunPage: undefined,
    city: undefined,
    building: undefined,
  });
});
