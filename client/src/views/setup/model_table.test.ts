// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The probed-model table's wiring, driven the way its seat drives it and
// with no look at all: whichever look draws the table, these are the
// rows it offers, what each press writes, and the rows handed back.

import { describe, expect, test } from "bun:test";

import { providerName } from "../../core/commands";
import type { ModelFact } from "../../core/probed";
import { Ceiling, Window } from "../../wire";
import type { FigureFrom, FiguresLine } from "../../wire";
import { chosenRows, figuresQuery, freshTable } from "./model_rows";
import { lookOf } from "./model_table";
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
  fact("vendor/writer", {
    contextTokens: 200_000,
    maxOutputTokens: 8192,
    inputPrice: "3",
    outputPrice: "15",
  }),
  fact("vendor/video-maker"),
  fact("vendor/seer", { inputModalities: ["text", "image"], contextTokens: 64_000 }),
  fact("vendor/painter", { inputModalities: ["image"] }),
];

// One row of the city's answer: a window and the rung that stated it,
// and a ceiling sent with its rung or left to the provider.
const line = (
  id: string,
  window: readonly [number, FigureFrom] | null,
  ceiling: readonly [number, FigureFrom] | "provider_default",
): FiguresLine => ({
  id,
  window: window === null ? null : { tokens: Window.make(window[0]), from: window[1] },
  ceiling:
    ceiling === "provider_default"
      ? ceiling
      : { sent: { tokens: Ceiling.make(ceiling[0]), from: ceiling[1] } },
});

// What the city answers for `SERVED` on the chat face of a server on
// this machine: the provider's list where it stated a figure, and
// otherwise an unknown window and the provider's own ceiling.
const CITY: readonly FiguresLine[] = [
  line("vendor/writer", [200_000, "upstream"], [8192, "upstream"]),
  line("vendor/video-maker", null, "provider_default"),
  line("vendor/seer", [64_000, "upstream"], "provider_default"),
  line("vendor/painter", null, "provider_default"),
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
    expect(ids(lookOf(state, SERVED, [], "en"))).toEqual(["vendor/seer", "vendor/writer"]);

    lookOf(state, SERVED, [], "en").manual.input("vendor/video-maker, typed/one");
    expect(ids(lookOf(state, SERVED, [], "en"))).toEqual([
      "typed/one",
      "vendor/seer",
      "vendor/video-maker",
      "vendor/writer",
    ]);

    lookOf(state, SERVED, [], "en").textOnly.toggle(false);
    expect(ids(lookOf(state, SERVED, [], "en"))).toEqual([
      "typed/one",
      "vendor/painter",
      "vendor/seer",
      "vendor/video-maker",
      "vendor/writer",
    ]);
  });

  test("the search narrows by id without regard to case", () => {
    const state = freshTable();
    lookOf(state, SERVED, [], "en").search.input("  WRIT ");
    expect(ids(lookOf(state, SERVED, [], "en"))).toEqual(["vendor/writer"]);
  });

  test("no row at all draws no table, only the count", () => {
    const look = lookOf(freshTable(), [], [], "en");
    expect(look.table).toBeUndefined();
    expect(look.count).toContain("0");
  });
});

describe("what a press writes and what is handed back", () => {
  test("an untouched box hands back no figure, so the city decides it and records the rung", () => {
    const state = freshTable();
    tick(lookOf(state, SERVED, [], "en"), "vendor/writer");
    expect(chosenRows(state, SERVED)).toEqual([
      {
        id: "vendor/writer",
        stated: { contextTokens: null, maxOutputTokens: null, input: null },
        tag: null,
      },
    ]);
  });

  test("typed figures, a stated input kind and a role reach the row; letters are no figure", () => {
    const state = freshTable();
    tick(lookOf(state, SERVED, [], "en"), "vendor/seer");
    const row = rowOf(lookOf(state, SERVED, [], "en"), "vendor/seer");
    row?.window.input("32768");
    row?.ceiling.input("lots");
    row?.input.pick("text_image");
    row?.role.pick("digest");
    expect(chosenRows(state, SERVED)).toEqual([
      {
        id: "vendor/seer",
        stated: { contextTokens: 32_768, maxOutputTokens: null, input: "text_image" },
        tag: "digest",
      },
    ]);

    row?.input.pick("");
    row?.role.pick("");
    expect(chosenRows(state, SERVED)).toEqual([
      {
        id: "vendor/seer",
        stated: { contextTokens: 32_768, maxOutputTokens: null, input: null },
        tag: null,
      },
    ]);
  });

  test("ticking all ticks the rows shown and leaves the hidden ones alone", () => {
    const state = freshTable();
    const ticks = lookOf(state, SERVED, [], "en").table?.tick;
    expect(ticks?.allPicked()).toBe(false);
    ticks?.onPickAll(true);
    expect(lookOf(state, SERVED, [], "en").table?.tick.allPicked()).toBe(true);
    expect(chosenRows(state, SERVED).map((row) => row.id)).toEqual([
      "vendor/writer",
      "vendor/seer",
    ]);
  });
});

describe("the columns", () => {
  test("the figure columns order a row that states nothing below every row that does", () => {
    const state = freshTable();
    lookOf(state, SERVED, CITY, "en").textOnly.toggle(false);
    const look = lookOf(state, SERVED, CITY, "en");
    const context = look.table?.columns.find((column) => column.key === "context");
    const rows = [...(look.table?.rows ?? [])]
      .sort(context?.compare ?? (() => 0))
      .map((row) => row.id);
    expect(rows).toEqual(["vendor/painter", "vendor/video-maker", "vendor/seer", "vendor/writer"]);
    expect(
      look.table?.columns
        .filter((column) => column.compare !== undefined)
        .map((column) => column.key),
    ).toEqual(["id", "context", "output"]);
  });

  test("a row's labels name the row, so two boxes in one column are told apart", () => {
    const look = lookOf(freshTable(), SERVED, [], "en");
    expect(rowOf(look, "vendor/seer")?.window.label).toEndWith("vendor/seer");
    expect(rowOf(look, "vendor/seer")?.role.label).toEndWith("vendor/seer");
    expect(rowOf(look, "vendor/writer")?.price).toBe("3 / 15");
    expect(rowOf(look, "vendor/seer")?.price).toBe("—");
  });
});

// The two hosts the city's ladder tells apart for one catalogue-known
// id: a relay off this machine, where the preset table states the
// vendor's window, and a server on this machine such as
// http://127.0.0.1:8811/v1, where no rung does (crates/gateway/Spec.lean
// §8-17; the city's side is `a_relay_forwarding_a_vendors_id_answers_the_vendors_window`
// in crates/accounting/src/views/providers.rs).
const RELAY = "https://relay.example.test/v1";
const SONNET = "claude-sonnet-4-5";

describe("what an untouched box says the city will register", () => {
  test("the question names the endpoint, the box text, the face, and each row's listed figures and role", () => {
    const state = freshTable();
    state.manual = "relay-hidden-1";
    state.role["vendor/writer"] = "main";
    expect(
      figuresQuery(
        { endpoint: "relay", baseUrl: RELAY, face: "open_ai" },
        state,
        SERVED.slice(0, 1),
      ),
    ).toEqual({
      model_figures: {
        endpoint: providerName("relay"),
        base_url: RELAY,
        face: "open_ai",
        models: [
          {
            id: "vendor/writer",
            context_tokens: Window.make(200_000),
            max_output_tokens: Ceiling.make(8192),
            tag: "main",
          },
          { id: "relay-hidden-1", context_tokens: null, max_output_tokens: null, tag: null },
        ],
      },
    });
  });

  test("a catalogue-known id reads the vendor's window behind a relay, and unknown on this machine", () => {
    const served = [fact(SONNET)];
    const atRelay = rowOf(
      lookOf(freshTable(), served, [line(SONNET, [200_000, "preset"], "provider_default")], "en"),
      SONNET,
    );
    expect(atRelay?.window).toMatchObject({
      placeholder: "200000",
      source: "from the city's catalogue",
    });
    expect(atRelay?.ceiling).toMatchObject({ placeholder: "", source: "the provider's default" });

    const atLoopback = rowOf(
      lookOf(freshTable(), served, [line(SONNET, null, "provider_default")], "en"),
      SONNET,
    );
    expect(atLoopback?.window).toMatchObject({
      placeholder: "",
      source: "unknown — the context reminder stays off",
    });
  });

  test("each rung says where the figure came from, and a box waiting on the city says nothing", () => {
    const served = [fact("a"), fact("b"), fact("c")];
    const answered = [
      line("a", [131_072, "upstream"], [16_384, "upstream"]),
      line("b", [50_000, "registered"], [64_000, "preset"]),
    ];
    const look = lookOf(freshTable(), served, answered, "en");
    expect([
      rowOf(look, "a")?.window,
      rowOf(look, "a")?.ceiling,
      rowOf(look, "b")?.window,
      rowOf(look, "b")?.ceiling,
    ]).toMatchObject([
      { placeholder: "131072", source: "from the provider's list" },
      { placeholder: "16384", source: "from the provider's list" },
      { placeholder: "50000", source: "as registered before" },
      { placeholder: "64000", source: "from the city's catalogue" },
    ]);
    expect(rowOf(look, "c")?.window).toMatchObject({ placeholder: "", source: "" });
  });

  test("a typed id the list does not carry is marked as named, and one the list carries is not", () => {
    const state = freshTable();
    state.manual = "relay-hidden-1, vendor/writer";
    const look = lookOf(state, SERVED, [], "en");
    expect([rowOf(look, "relay-hidden-1")?.named, rowOf(look, "vendor/writer")?.named]).toEqual([
      "named by you, not listed by the endpoint",
      undefined,
    ]);
  });
});
