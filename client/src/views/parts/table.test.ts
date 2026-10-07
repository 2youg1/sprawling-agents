// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a table decides and hands its look, against the table paragraph
// of `client/spec/Views/Parts.lean` §7-6: `aria-sort` only on a column
// that can be ordered by, the names of the boxes and the corrected
// cells, and the header box's third state while some rows are ticked.

import { describe, expect, test } from "bun:test";
import { createRawSnippet } from "svelte";

import { UNSORTED, lookOf, mixed, ordered, turned, type Column, type Selection, type Sorted } from "./table";

interface Model {
  readonly id: string;
  readonly context: number;
}

const FIRST: Model = { id: "b", context: 200 };

const MODELS: readonly Model[] = [
  FIRST,
  { id: "a", context: 8 },
  { id: "c", context: 32 },
];

// A cell's drawing is never called here: what the look does with it is
// measured in the gallery.
const drawn = createRawSnippet<[Model]>(() => ({ render: () => "<span></span>" }));

const ID: Column<Model> = { key: "id", header: "Model", render: drawn };
const CONTEXT: Column<Model> = {
  key: "context",
  header: "Context",
  render: drawn,
  compare: (left, right) => left.context - right.context,
  editable: { text: (row) => String(row.context), onEdit: () => undefined },
};

function choosing(picked: readonly string[], all: boolean, record: string[] = []): Selection<Model> {
  return {
    picked: (row) => picked.includes(row.id),
    onPick: (row, on) => record.push(`${row.id}:${String(on)}`),
    allLabel: "All models",
    onPickAll: (on) => record.push(`all:${String(on)}`),
    allPicked: () => all,
  };
}

function lookAt(sorted: Sorted, selection?: Selection<Model>) {
  return lookOf(
    {
      caption: "Models",
      columns: [ID, CONTEXT],
      rows: MODELS,
      keyOf: (row) => row.id,
      ...(selection === undefined ? {} : { selection }),
    },
    sorted,
    () => undefined,
  );
}

describe("ordering", () => {
  test("the same column turns round, another column starts ascending", () => {
    const once = turned(UNSORTED, "context");
    expect([once, turned(once, "context"), turned(turned(once, "context"), "id")]).toEqual([
      { by: "context", order: "up" },
      { by: "context", order: "down" },
      { by: "id", order: "up" },
    ]);
  });

  test("rows keep their order until a sortable column is chosen", () => {
    const ids = (sorted: Sorted) => ordered(MODELS, [ID, CONTEXT], sorted).map((row) => row.id);
    expect([
      ids(UNSORTED),
      ids({ by: "id", order: "up" }),
      ids({ by: "context", order: "up" }),
      ids({ by: "context", order: "down" }),
    ]).toEqual([
      ["b", "a", "c"],
      ["b", "a", "c"],
      ["a", "c", "b"],
      ["b", "c", "a"],
    ]);
  });
});

describe("the headers", () => {
  test("aria-sort is written only on the column that can be ordered by", () => {
    expect(lookAt(UNSORTED).heads.map((head) => head.cell)).toEqual([{}, { "aria-sort": "none" }]);
    expect(lookAt({ by: "context", order: "down" }).heads.map((head) => head.cell)).toEqual([
      {},
      { "aria-sort": "descending" },
    ]);
  });

  test("a sortable header is a button that turns its own column", () => {
    const pressed: string[] = [];
    const look = lookOf(
      { caption: "Models", columns: [ID, CONTEXT], rows: MODELS, keyOf: (row) => row.id },
      UNSORTED,
      (key) => pressed.push(key),
    );
    look.heads.forEach((head) => head.sort?.onclick());
    expect([look.heads.map((head) => head.sort?.type), pressed]).toEqual([[undefined, "button"], ["context"]]);
  });
});

describe("the boxes", () => {
  test("the header box is in its third state exactly while some but not all rows are ticked", () => {
    expect([
      mixed(choosing([], false), MODELS),
      mixed(choosing(["a"], false), MODELS),
      mixed(choosing(["a", "b", "c"], true), MODELS),
    ]).toEqual([false, true, false]);
    expect(lookAt(UNSORTED, choosing(["a"], false)).all?.indeterminate).toBe(true);
  });

  test("each box is named and hands back what was ticked", () => {
    const record: string[] = [];
    const look = lookAt(UNSORTED, choosing(["a"], false, record));
    look.all?.onchange({ currentTarget: { checked: true } });
    look.lines[1]?.tick?.onchange({ currentTarget: { checked: false } });
    expect([look.all?.["aria-label"], look.lines.map((line) => line.tick?.["aria-label"]), record]).toEqual([
      "All models",
      ["b", "a", "c"],
      ["all:true", "a:false"],
    ]);
  });

  test("a table without a selection draws no boxes", () => {
    const look = lookAt(UNSORTED);
    expect([look.all, look.lines.map((line) => line.tick)]).toEqual([undefined, [undefined, undefined, undefined]]);
  });
});

describe("the cells", () => {
  test("a corrected column is a named input that hands back what was typed", () => {
    const edits: string[] = [];
    const column: Column<Model> = {
      ...CONTEXT,
      editable: { text: (row) => String(row.context), onEdit: (row, text) => edits.push(`${row.id}=${text}`) },
    };
    const look = lookOf(
      { caption: "Models", columns: [ID, column], rows: MODELS, keyOf: (row) => row.id },
      UNSORTED,
      () => undefined,
    );
    const held = look.lines[0]?.cells[1]?.held;
    expect(held?.kind).toBe("field");
    if (held?.kind !== "field") return;
    held.field.onchange({ currentTarget: { value: "256" } });
    expect([held.field["aria-label"], held.field.value, edits]).toEqual(["Context b", "200", ["b=256"]]);
  });

  test("any other cell hands the column's drawing its row", () => {
    expect(lookAt(UNSORTED).lines[0]?.cells[0]?.held).toEqual({ kind: "drawn", render: drawn, row: FIRST });
  });
});
