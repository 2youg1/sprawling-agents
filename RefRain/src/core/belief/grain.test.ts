// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { get } from "../store";

import { createBelief } from "../belief";
import { runText } from "./runs";
import type { CityAnswer, RunSummary } from "../../wire";
import { RunId, Seq } from "../../wire";

function listed(run: RunId): RunSummary {
  return {
    run,
    addr: null,
    started: null,
    last_seq: Seq.make(1),
    last_kind: "run_started",
    frozen: false,
    who: "hall/mayor",
  };
}

test("a delta publishes only its run and field and preserves the whole belief", () => {
  const store = createBelief(() => 0);
  const own = RunId.make("00000000-0000-4000-8000-000000000001");
  const other = RunId.make("00000000-0000-4000-8000-000000000002");
  const city: CityAnswer = {
    active: 2,
    buildings: [],
    frozen: 0,
    halted: [],
    pursuits: [],
    runs: [listed(own), listed(other)],
  };
  store.adoptCity(city);
  const { runs } = get(store.belief);
  const writer = runs[own];
  const distant = runs[other];
  if (writer === undefined || distant === undefined)
    return expect.unreachable("The runs were not adopted");
  store.say({ run: own, increment: { said: "initial" } });

  const spoken: string[] = [];
  const thoughts: string[] = [];
  const elsewhere: string[] = [];
  const belief: (typeof runs)[] = [];
  const stopSpoken = runText(writer, "saying").subscribe((value) => {
    spoken.push(value);
  });
  const stopThought = runText(writer, "thinking").subscribe((value) => {
    thoughts.push(value);
  });
  const stopElsewhere = runText(distant, "saying").subscribe((value) => {
    elsewhere.push(value);
  });
  const stopBelief = store.belief.subscribe((value) => {
    belief.push(value.runs);
  });
  store.say({ run: own, increment: { said: "x" } });
  store.say({ run: own, increment: { thought: "y" } });
  stopSpoken();
  stopThought();
  stopElsewhere();
  stopBelief();
  store.say({ run: own, increment: { said: "z" } });

  expect(spoken).toEqual(["initial", "initialx"]);
  expect(thoughts).toEqual(["", "y"]);
  expect(elsewhere).toEqual([""]);
  expect(belief).toEqual([runs]);
  expect(get(runText(writer, "saying"))).toBe("initialxz");
});
