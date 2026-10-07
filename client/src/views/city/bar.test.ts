// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { MAYOR, toFragment } from "../../core/route";
import { barLookOf } from "./bar";

const QUIET = {
  approvals_waiting: 0,
  buildings: 3,
  discards_outstanding: 0,
  events: 1_204,
  runs_active: 2,
  runs_frozen: 5,
  signals_waiting: 0,
};

describe("the city's row of figures", () => {
  // Red on the bar means somebody has to answer something: a queue the
  // person owns is in the alert tone only while it holds anything, and a
  // figure the city is getting on with never is, however large.
  test("a queue the person owns turns alert only above zero", () => {
    const tones = (metrics: typeof QUIET) =>
      barLookOf(metrics, null, "en")?.readings.map((reading) => [reading.key, reading.tone]);
    expect(tones(QUIET)).toEqual([
      ["metric_events", "plain"],
      ["metric_runs_active", "plain"],
      ["metric_runs_frozen", "plain"],
      ["metric_approvals", "plain"],
      ["metric_signals", "plain"],
      ["metric_discards", "plain"],
    ]);
    expect(tones({ ...QUIET, approvals_waiting: 1, signals_waiting: 2 })).toEqual([
      ["metric_events", "plain"],
      ["metric_runs_active", "plain"],
      ["metric_runs_frozen", "plain"],
      ["metric_approvals", "alert"],
      ["metric_signals", "alert"],
      ["metric_discards", "plain"],
    ]);
  });

  // Each figure links to the page a person acts on it from, and the
  // spend closes the row with a link to the cost page.
  test("figures link where they are acted on, and the spend closes the row", () => {
    const look = barLookOf(QUIET, 2_500_000, "en");
    expect(look?.readings.map((reading) => reading.wire?.href ?? null)).toEqual([
      toFragment({ kind: "record", lens: "ledger" }),
      null,
      null,
      toFragment({ kind: "talk", address: MAYOR }),
      null,
      toFragment({ kind: "record", lens: "bin" }),
      toFragment({ kind: "cost" }),
    ]);
    expect(look?.readings.at(-1)).toEqual({
      key: "cost_total",
      label: "spent in all",
      value: "$2.50",
      tone: "plain",
      wire: { href: toFragment({ kind: "cost" }) },
    });
  });

  test("before the city answers either question there is no bar", () => {
    expect(barLookOf(null, null, "en")).toBeNull();
    expect(barLookOf(null, 0, "en")?.readings.map((reading) => reading.key)).toEqual(["cost_total"]);
  });
});
