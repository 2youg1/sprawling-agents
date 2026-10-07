// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "../../core/belief";
import type { Doing } from "../../core/doing";
import { toFragment } from "../../core/route";
import { Address, RunId, Seq, TimeMs } from "../../wire";
import { resultsLookOf } from "./results";

function run(index: number, doing: Doing, more: Partial<RunBelief> = {}): RunBelief {
  return {
    run: RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`),
    addr: Address.make("docs/zh"),
    started: null,
    task: `task ${String(index)}`,
    goal: null,
    lastSeq: Seq.make(1),
    doing,
    model: null,
    pr: null,
    ask: null,
    local: false,
    saying: "",
    thinking: "",
    ...more,
  };
}

describe("the results city's rows", () => {
  // The end of a row is the one fact about how the run ended a person
  // acts on: a finished run's produce and pull request, a waiting run's
  // question, a failed run's reason - and nothing where there is none.
  test("each row ends in what its outcome has to say, and links to its run", () => {
    const done = run(1, { kind: "frozen", completion: "done" }, { pr: "fix/totals", started: TimeMs.make(0) });
    const asks = run(2, { kind: "waiting" }, { ask: "write to docs/zh" });
    const silent = run(3, { kind: "waiting" });
    const failed = run(4, { kind: "frozen", completion: "limit" });
    const look = resultsLookOf([{ recency: "minutes", runs: [done, asks, silent, failed] }], "en");
    expect(look.none).toBeNull();
    expect(look.bands[0]?.count).toBe(4);
    expect(look.bands[0]?.rows.map((row) => row.tail)).toEqual([
      { kind: "produced", run: done.run, pr: "PR fix/totals" },
      { kind: "asks", text: "asks to write to docs/zh" },
      { kind: "none" },
      { kind: "stopped", text: "limit" },
    ]);
    expect(look.bands[0]?.rows[0]?.wire.href).toBe(toFragment({ kind: "run", run: done.run }));
  });

  test("a filter that keeps no run says so", () => {
    expect(resultsLookOf([], "en")).toEqual({ bands: [], none: "nothing here" });
  });
});
