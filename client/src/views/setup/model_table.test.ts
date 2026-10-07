// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The probed-model table's wiring, driven the way its seat drives it and
// with no look at all: whichever look draws the table, these are the
// rows it offers, what each press writes, and the rows handed back.

import { describe, expect, test } from "bun:test";

import type { ModelFact } from "../../core/probed";
import { chosenRows, freshTable, lookOf } from "./model_table";
import type { ModelRowLook, ModelTableLook } from "./model_table";

const fact = (id: string, over: Partial<ModelFact> = {}): ModelFact => ({
  id,
  contextTokens: null,
  maxOutputTokens: null,
  inputModalities: [],
  inputPrice: null,
  outputPrice: null,
  ...over,
});

const SERVED: readonly ModelFact[] = [
  fact("vendor/writer", { contextTokens: 200_000, maxOutputTokens: 8192, inputPrice: "3", outputPrice: "15" }),
  fact("vendor/video-maker"),
  fact("vendor/seer", { inputModalities: ["text", "image"], contextTokens: 64_000 }),
  fact("vendor/painter", { inputModalities: ["image"] }),
];

const rowOf = (look: ModelTableLook, id: string): ModelRowLook | undefined =>
  look.table?.rows.find((row) => row.id === id);

const ids = (look: ModelTableLook) => look.table?.rows.map((row) => row.id);

// Ticks one row through the tick column, as a press on its checkbox does.
function tick(look: ModelTableLook, id: string): void {
  const row = rowOf(look, id);
  if (row !== undefined) look.table?.tick.onPick(row, true);
}

describe("which rows the table offers", () => {
  test("the text filter hides what a provider or a name says is not text, and a typed id stays", () => {
    const state = freshTable();
    expect(ids(lookOf(state, SERVED, "en"))).toEqual(["vendor/seer", "vendor/writer"]);

    lookOf(state, SERVED, "en").manual.input("vendor/video-maker, typed/one");
    expect(ids(lookOf(state, SERVED, "en"))).toEqual(["typed/one", "vendor/seer", "vendor/video-maker", "vendor/writer"]);

    lookOf(state, SERVED, "en").textOnly.toggle(false);
    expect(ids(lookOf(state, SERVED, "en"))).toEqual([
      "typed/one",
      "vendor/painter",
      "vendor/seer",
      "vendor/video-maker",
      "vendor/writer",
    ]);
  });

  test("the search narrows by id without regard to case", () => {
    const state = freshTable();
    lookOf(state, SERVED, "en").search.input("  WRIT ");
    expect(ids(lookOf(state, SERVED, "en"))).toEqual(["vendor/writer"]);
  });

  test("no row at all draws no table, only the count", () => {
    const look = lookOf(freshTable(), [], "en");
    expect(look.table).toBeUndefined();
    expect(look.count).toContain("0");
  });
});

describe("what a press writes and what is handed back", () => {
  test("an untouched box shows the provider's figure as a placeholder and hands it back", () => {
    const state = freshTable();
    tick(lookOf(state, SERVED, "en"), "vendor/writer");
    expect(rowOf(lookOf(state, SERVED, "en"), "vendor/writer")?.window).toMatchObject({ value: "", placeholder: "200000" });
    expect(chosenRows(state, SERVED)).toEqual([
      { id: "vendor/writer", stated: { contextTokens: 200_000, maxOutputTokens: 8192, input: null }, tag: null },
    ]);
  });

  test("typed figures, a stated input kind and a role reach the row; letters are no figure", () => {
    const state = freshTable();
    tick(lookOf(state, SERVED, "en"), "vendor/seer");
    const row = rowOf(lookOf(state, SERVED, "en"), "vendor/seer");
    row?.window.input("32768");
    row?.ceiling.input("lots");
    row?.input.pick("text_image");
    row?.role.pick("digest");
    expect(chosenRows(state, SERVED)).toEqual([
      { id: "vendor/seer", stated: { contextTokens: 32_768, maxOutputTokens: null, input: "text_image" }, tag: "digest" },
    ]);
    expect(lookOf(state, SERVED, "en").needed).toBeDefined();

    row?.input.pick("");
    row?.role.pick("");
    expect(chosenRows(state, SERVED)).toEqual([
      { id: "vendor/seer", stated: { contextTokens: 32_768, maxOutputTokens: null, input: null }, tag: null },
    ]);
  });

  test("ticking all ticks the rows shown and leaves the hidden ones alone", () => {
    const state = freshTable();
    const ticks = lookOf(state, SERVED, "en").table?.tick;
    expect(ticks?.allPicked()).toBe(false);
    ticks?.onPickAll(true);
    expect(lookOf(state, SERVED, "en").table?.tick.allPicked()).toBe(true);
    expect(chosenRows(state, SERVED).map((row) => row.id)).toEqual(["vendor/writer", "vendor/seer"]);
  });
});

describe("the columns", () => {
  test("the figure columns order a row that states nothing below every row that does", () => {
    const state = freshTable();
    lookOf(state, SERVED, "en").textOnly.toggle(false);
    const look = lookOf(state, SERVED, "en");
    const context = look.table?.columns.find((column) => column.key === "context");
    const rows = [...(look.table?.rows ?? [])].sort(context?.compare ?? (() => 0)).map((row) => row.id);
    expect(rows).toEqual(["vendor/painter", "vendor/video-maker", "vendor/seer", "vendor/writer"]);
    expect(look.table?.columns.filter((column) => column.compare !== undefined).map((column) => column.key)).toEqual([
      "id",
      "context",
      "output",
    ]);
  });

  test("a row's labels name the row, so two boxes in one column are told apart", () => {
    const look = lookOf(freshTable(), SERVED, "en");
    expect(rowOf(look, "vendor/seer")?.window.label).toEndWith("vendor/seer");
    expect(rowOf(look, "vendor/seer")?.role.label).toEndWith("vendor/seer");
    expect(rowOf(look, "vendor/writer")?.price).toBe("3 / 15");
    expect(rowOf(look, "vendor/seer")?.price).toBe("—");
  });
});
