// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { flushSync } from "svelte";
import { get } from "svelte/store";

import { createBelief } from "../belief";
import type { CityAnswer, RunSummary } from "../../wire";
import { RunId, Seq } from "../../wire";

function listed(run: RunId): RunSummary {
  return { run, addr: null, started: null, last_seq: Seq.make(1), last_kind: "run_started", frozen: false, who: "hall/mayor" };
}

// A view reads one run's words through the table; a token for another
// run is not its business, so it must not run again for it.
test("a delta invalidates only its own run", () => {
  const store = createBelief(() => 0);
  const own = RunId.make("00000000-0000-4000-8000-000000000001");
  const other = RunId.make("00000000-0000-4000-8000-000000000002");
  const city: CityAnswer = { active: 2, buildings: [], frozen: 0, halted: [], pursuits: [], runs: [listed(own), listed(other)] };
  store.adoptCity(city);
  const runs = get(store.belief).runs;
  const seen: { own: (string | undefined)[]; other: (string | undefined)[] } = { own: [], other: [] };
  const stop = $effect.root(() => {
    $effect(() => {
      seen.own.push(runs[own]?.saying);
    });
    $effect(() => {
      seen.other.push(runs[other]?.saying);
    });
  });
  flushSync();
  store.say({ run: own, increment: { said: "x" } });
  flushSync();
  stop();
  expect(seen).toEqual({ own: ["", "x"], other: [""] });
});
