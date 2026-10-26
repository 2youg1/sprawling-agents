// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The draft of the capabilities section: a row is written only when it
// departs from what it inherits, a group press moves every member, a
// fixed row never moves, and the meter is what moves into the core less
// what moves out. The vectors are the ones `#eval` prints in
// client/spec/Views/Catalog.lean, replayed through `pressRow` and
// `pressGroup`; a difference between the model and the code turns the
// last test red.

import { describe, expect, test } from "bun:test";

import type { CatalogAnswer, Exposure } from "../../wire";
import { Address } from "../../wire";
import { draftOf, effectiveOf, meterOf, pressGroup, pressRow, rowsOf, sameTable } from "./catalog";
import type { Row, Table } from "./catalog";

// The model's fixture, tool `n` spelled `t<n>` and namespace `n` spelled
// `n<n>`.
function row(
  name: number,
  ns: number,
  bytes: number,
  fixed: boolean,
  inherited: Exposure,
  nearer?: Exposure,
): Row {
  return {
    tool: {
      name: `t${String(name)}`,
      namespace: `n${String(ns)}`,
      hint: "",
      bytes,
      exposure: inherited,
      from: fixed ? "fixed" : "default",
      fallback: inherited,
      runs_used: 0,
      last_used: null,
    },
    fixed,
    inherited,
    nearer,
  };
}

const ROWS: readonly Row[] = [
  row(0, 0, 100, false, "core"),
  row(1, 0, 50, false, "dormant"),
  row(2, 1, 30, false, "dormant"),
  row(3, 1, 20, true, "core"),
  row(4, 1, 40, false, "dormant", "off"),
];

const EMPTY: Table = { tools: {}, namespaces: {} };

type Edit =
  { readonly row: readonly [number, Exposure] } | { readonly group: readonly [number, Exposure] };

function apply(table: Table, edit: Edit): Table {
  if ("row" in edit) {
    const [name, value] = edit.row;
    const at = ROWS.find((each) => each.tool.name === `t${String(name)}`);
    return at === undefined ? table : pressRow(table, at, value);
  }
  const [ns, value] = edit.group;
  return pressGroup(ROWS, table, `n${String(ns)}`, value);
}

describe("the capabilities draft", () => {
  test("a fixed row stays core and a press on it changes nothing", () => {
    const fixed = ROWS.filter((each) => each.fixed);
    expect(fixed.map((each) => [effectiveOf(EMPTY, each), pressRow(EMPTY, each, "off")])).toEqual([
      ["core", EMPTY],
    ]);
  });

  test("the building's rows inherit the city's table, and the city's rows yield to the building's", () => {
    const answer: CatalogAnswer = {
      building: Address.make("shop"),
      run: null,
      core_bytes: 0,
      index_bytes: 0,
      runs_seen: 0,
      tools: [{ ...row(0, 0, 10, false, "dormant").tool, name: "plan", namespace: "sprawling" }],
      city: { tools: { plan: "core" } },
      stated: { namespaces: { sprawling: "off" } },
    };
    expect(
      rowsOf(answer, "building").map((each) => [
        effectiveOf(draftOf(answer, "building"), each),
        each.inherited,
      ]),
    ).toEqual([["off", "core"]]);
    expect(
      rowsOf(answer, "city").map((each) => [
        effectiveOf(draftOf(answer, "city"), each),
        each.nearer,
      ]),
    ).toEqual([["off", "off"]]);
    expect(sameTable(draftOf(answer, "city"), { tools: { plan: "core" }, namespaces: {} })).toBe(
      true,
    );
  });

  // Each vector: the edits, the table they leave (keys and values, in no
  // order), every row's exposure, and the meter against the empty table,
  // as `#eval vectors.map …` printed them.
  const VECTORS: readonly {
    readonly edits: readonly Edit[];
    readonly tools: Readonly<Record<string, Exposure>>;
    readonly namespaces: Readonly<Record<string, Exposure>>;
    readonly rows: readonly Exposure[];
    readonly meter: number;
  }[] = [
    {
      edits: [{ row: [1, "core"] }],
      tools: { t1: "core" },
      namespaces: {},
      rows: ["core", "core", "dormant", "core", "off"],
      meter: 50,
    },
    {
      edits: [{ row: [1, "core"] }, { row: [1, "dormant"] }],
      tools: {},
      namespaces: {},
      rows: ["core", "dormant", "dormant", "core", "off"],
      meter: 0,
    },
    {
      edits: [{ row: [0, "off"] }, { row: [3, "off"] }],
      tools: { t0: "off" },
      namespaces: {},
      rows: ["off", "dormant", "dormant", "core", "off"],
      meter: -100,
    },
    {
      edits: [{ group: [1, "core"] }],
      tools: {},
      namespaces: { n1: "core" },
      rows: ["core", "dormant", "core", "core", "off"],
      meter: 30,
    },
    {
      edits: [{ group: [1, "core"] }, { row: [2, "dormant"] }],
      tools: { t2: "dormant" },
      namespaces: { n1: "core" },
      rows: ["core", "dormant", "dormant", "core", "off"],
      meter: 0,
    },
    {
      edits: [{ group: [0, "off"] }, { row: [0, "core"] }],
      tools: { t0: "core" },
      namespaces: { n0: "off" },
      rows: ["core", "off", "dormant", "core", "off"],
      meter: 0,
    },
    {
      edits: [{ group: [1, "dormant"] }],
      tools: {},
      namespaces: {},
      rows: ["core", "dormant", "dormant", "core", "off"],
      meter: 0,
    },
  ];

  test("follows the model on every edit sequence it prints", () => {
    for (const vector of VECTORS) {
      const table = vector.edits.reduce(apply, EMPTY);
      expect({
        tools: table.tools,
        namespaces: table.namespaces,
        rows: ROWS.map((each) => effectiveOf(table, each)),
        meter: meterOf(EMPTY, table, ROWS),
      }).toEqual({
        tools: vector.tools,
        namespaces: vector.namespaces,
        rows: [...vector.rows],
        meter: vector.meter,
      });
    }
  });
});
