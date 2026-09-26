// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { flushSync } from "svelte";
import { get } from "svelte/store";

import { createBelief } from "../../core/belief";
import type { CityAnswer } from "../../wire";
import { Address, RunId, Seq, TimeMs } from "../../wire";
import { boardRuns } from "./lineage";

// The city route rebuilds the board from the run table, and a model
// streaming words into one run writes that run's `saying` for every
// token: the board draws none of it, so a token must not rebuild a
// board of a thousand rows, and a board run carries only what it draws.
test("the board reads what it draws from the run table and no token wakes it", () => {
  const store = createBelief(() => 0);
  const run = RunId.make("00000000-0000-4000-8000-000000000001");
  const city: CityAnswer = {
    active: 1,
    buildings: [],
    frozen: 0,
    halted: [],
    pursuits: [],
    runs: [{ run, addr: Address.make("web/api"), started: TimeMs.make(5), last_seq: Seq.make(1), last_kind: "run_started", frozen: false, who: "hall/mayor" }],
  };
  store.adoptCity(city);
  const runs = get(store.belief).runs;
  const built: unknown[] = [];
  const stop = $effect.root(() => {
    $effect(() => {
      built.push(boardRuns(runs));
    });
  });
  flushSync();
  store.say({ run, increment: { said: "x" } });
  flushSync();
  stop();
  expect(built).toEqual([[{ run, addr: "web/api", task: null, started: 5, ended: null, doing: { kind: "thinking" } }]]);
});
