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

import type { CatalogAnswer, CatalogSkill, Exposure } from "../../wire";
import { Address, B3Hash, ServerLabel } from "../../wire";
import {
  acceptSkill,
  acceptTool,
  draftOf,
  effectiveOf,
  keepSkill,
  meterOf,
  pressGroup,
  pressRow,
  pressSkill,
  rowsOf,
  sameTable,
  skillMeterOf,
} from "./catalog";
import type { Row, Selection, Table } from "./catalog";

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
      current: {
        bytes,
        hint: "",
        description: "",
        digest: B3Hash.make(name.toString(16).padStart(64, "0")),
        input_schema: {},
      },
      pinned: null,
      change: null,
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

const EMPTY: Table = { tools: {}, namespaces: {}, skills: {} };

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
      reading_bytes: 0,
      always: 0,
      dormant: 0,
      skills: 0,
      changed: 0,
      open_sessions: 0,
      runs_seen: 0,
      skill_rows: [],
      references: [],
      skill_pins: {},
      reading_room: [],
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
    expect(
      sameTable(draftOf(answer, "city"), { tools: { plan: "core" }, namespaces: {}, skills: {} }),
    ).toBe(true);
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

  test("tool edits leave the skill positions alone", () => {
    const first = ROWS.find((row) => row.tool.name === "t0");
    expect(first).toBeDefined();
    if (first === undefined) return;
    const table: Table = { ...EMPTY, skills: { how: "core" } };
    expect(pressRow(table, first, "off").skills).toEqual({ how: "core" });
    expect(pressGroup(ROWS, table, "n0", "off").skills).toEqual({ how: "core" });
  });

  test("a new skill starts on call with its candidate pinned", () => {
    const candidate = B3Hash.make("1".padStart(64, "0"));
    const skill: CatalogSkill = {
      name: "how",
      section: "engineering",
      disclosure: "how",
      current_document: "# How\n",
      pinned_document: null,
      candidate,
      pinned: null,
      change: "added",
      exposure: "off",
      from: "default",
      audit: null,
      audit_stale: false,
      runs_used: 0,
      last_used: null,
    };
    const next = pressSkill(
      { catalog: EMPTY, references: [], skillPins: {}, readingRoom: [] },
      { skill, inherited: "dormant", nearer: undefined },
      "dormant",
      "building",
    );
    expect(next.catalog.skills).toEqual({});
    expect(next.readingRoom).toEqual(["how"]);
    expect(next.skillPins).toEqual({ how: candidate });
  });

  test("Accept moves only the intended pin and Keep restores it", () => {
    const old = B3Hash.make("1".padStart(64, "0"));
    const current = B3Hash.make("2".padStart(64, "0"));
    const skill: CatalogSkill = {
      name: "how",
      section: "engineering",
      disclosure: "how",
      current_document: "# New\n",
      pinned_document: "# Old\n",
      candidate: current,
      pinned: old,
      change: "changed",
      exposure: "dormant",
      from: "default",
      audit: "pass",
      audit_stale: false,
      runs_used: 0,
      last_used: null,
    };
    const selection = {
      catalog: EMPTY,
      references: [
        {
          label: ServerLabel.make("n0"),
          definition: old,
          tools: { t0: old },
        },
      ],
      skillPins: { how: old },
      readingRoom: ["how"],
    };
    const acceptedSkill = acceptSkill(selection, skill);
    expect(acceptedSkill.skillPins).toEqual({ how: current });
    expect(keepSkill(acceptedSkill, skill).skillPins).toEqual({ how: old });
    const first = ROWS.find((row) => row.tool.name === "t0");
    expect(first).toBeDefined();
    if (first === undefined) return;
    const digest = first.tool.current?.digest;
    expect(digest).toBeDefined();
    if (digest === undefined) return;
    expect(acceptTool(selection, first.tool).references[0]?.tools).toEqual({ t0: digest });
  });

  test("Accept counts the new version's bytes, not the old version's twice", () => {
    const first = ROWS.find((row) => row.tool.name === "t0");
    expect(first).toBeDefined();
    if (first?.tool.current == null) return;
    const current = { ...first.tool.current, bytes: 120 };
    const pinned = { ...first.tool.current, bytes: 100 };
    const table: Table = { ...EMPTY, tools: { t0: "core" } };
    expect(meterOf(table, table, [{ ...first, tool: { ...first.tool, current, pinned } }])).toBe(
      20,
    );

    const skill: CatalogSkill = {
      name: "how",
      section: "engineering",
      disclosure: "how",
      current_document: "new version\n",
      pinned_document: "old\n",
      candidate: current.digest,
      pinned: pinned.digest,
      change: "changed",
      exposure: "core",
      from: "default",
      audit: null,
      audit_stale: false,
      runs_used: 0,
      last_used: null,
    };
    const selection: Selection = {
      catalog: { ...EMPTY, skills: { how: "core" } },
      references: [],
      skillPins: {},
      readingRoom: ["how"],
    };
    expect(
      skillMeterOf(selection, selection, [{ skill, inherited: "core", nearer: undefined }]),
    ).toBe(
      new TextEncoder().encode("new version\n").length - new TextEncoder().encode("old\n").length,
    );
  });

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
