// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The model picker checked against `client/spec/Views/Picker.lean`: the
// window of every list of up to nine entries with every entry pinned,
// and the room each section keeps over every state an open picker
// reaches without typing, on cities generated across both orders.

import { describe, expect, test } from "bun:test";

import type { Effort, EndpointSummary, ModelFactsSummary } from "../../wire";
import { Window } from "../../wire";
import type { Entry, Section } from "./picker";
import { SECTION, SHOWN, drawnRows, firstLevel, offersOf, orderOf, roomOf, usedLevel, windowed } from "./picker";
import type { PickerHands } from "./picker_look";
import type { PickerHeld } from "./picker_scene";
import { pickerOf } from "./picker_look";

const entries = (count: number): readonly Entry[] => Array.from({ length: count }, (_unused, at) => ({ id: `e${String(at)}`, offers: [] }));

describe("a list's window", () => {
  test("holds at most five entries and the pinned one (the_window_holds_five, the_pinned_entry_is_shown)", () => {
    const wrong: string[] = [];
    for (let count = 0; count <= 9; count += 1)
      for (let pin = 0; pin < Math.max(1, count); pin += 1) {
        const all = entries(count);
        const view = windowed(all, `e${String(pin)}`, false);
        const shown = view.entries.map((each) => each.id);
        if (shown.length > SHOWN || (count > 0 && !shown.includes(`e${String(pin)}`)) || view.more !== Math.max(0, count - SHOWN))
          wrong.push(`${String(count)} pinned ${String(pin)}: ${shown.join(",")} +${String(view.more)}`);
        if (drawnRows(count) > SHOWN + 1) wrong.push(`${String(count)} draws ${String(drawnRows(count))}`);
      }
    expect(wrong).toEqual([]);
  });
});

function facts(id: string, canonical: string, levels: readonly Effort[]): ModelFactsSummary {
  return {
    id,
    canonical,
    context_tokens: Window.make(128_000),
    input_modalities: [],
    input_price: null,
    output_price: null,
    thinking: { levels: [...levels], on: "unknown", from: "upstream", words: [] },
  };
}

function endpoint(name: string, models: readonly ModelFactsSummary[]): EndpointSummary {
  return {
    account_status: [],
    base_url: "http://127.0.0.1:1/v1",
    connection_kind: "openai_compat",
    dialect: "open_ai",
    has_credential: true,
    label: name,
    local: false,
    models: [...models],
    name,
    tuning: { headers: [], overrides: [], proxying: "except_local" },
  };
}

// Cities across both orders: provider `p<i>` serves models `m<j>` for
// every j below its own count, so the shared models group by canonical,
// and every third model has no thinking control.
function city(counts: readonly number[]): readonly EndpointSummary[] {
  return counts.map((count, at) =>
    endpoint(
      `p${String(at)}`,
      Array.from({ length: count }, (_unused, j) => facts(`p${String(at)}/m${String(j)}`, `m${String(j)}`, j % 3 === 2 ? [] : ["low", "high"])),
    ),
  );
}

const nothing = (): (() => void) => () => undefined;
const HANDS: PickerHands = {
  open: nothing,
  close: nothing,
  hold: nothing,
  keep: nothing,
  bound: nothing,
  cursor: nothing,
  focusFilter: nothing,
  holdToken: nothing,
  holdFilter: nothing,
  holdFrame: nothing,
};

const held = (parent: string | null, pick: PickerHeld["pick"], whole: readonly Section[]): PickerHeld => ({
  open: true,
  segment: null,
  pick,
  parent,
  pinned: { first: undefined, second: undefined },
  opened: null,
  query: "",
  whole,
  kept: [],
  binding: null,
  active: null,
});

describe("the room an open picker keeps", () => {
  test("no state reached without typing outgrows it (no_reachable_state_outgrows_the_open_height)", () => {
    const wrong: string[] = [];
    for (const counts of [[1], [2, 1], [1, 1, 1], [7, 1], [3, 8, 2], [1, 1, 1, 1, 1, 1, 1], [12]]) {
      const endpoints = city(counts);
      const offers = offersOf(endpoints);
      const order = orderOf(offers);
      const room = roomOf(order, offers);
      const wholes: readonly (readonly Section[])[] = [[], [SECTION.first], [SECTION.second], [SECTION.first, SECTION.second]];
      for (const parent of firstLevel(order, offers))
        for (const offer of parent.offers)
          for (const whole of wholes) {
            const menu = pickerOf("en", { endpoints, chosen: undefined, stated: null, pick: { model: nothing(), level: nothing() } }, held(parent.id, offer, whole), HANDS)?.menu;
            const rows = (section: Section): number => (menu?.columns.find((column) => column.id === section)?.rows.length ?? 0) + (menu?.sections[section]?.spare ?? 0);
            const drawn = (section: Section, kept: number): number => (menu?.sections[section]?.scrolls === true ? kept : rows(section));
            const band = menu?.columns.some((column) => column.id === SECTION.thinking) === true || menu?.bandRoom === true;
            const state = `${JSON.stringify(counts)} ${parent.id} ${offer.model} ${whole.join("+")}`;
            if (drawn(SECTION.first, room.first) !== room.first) wrong.push(`${state}: first ${String(rows(SECTION.first))} of ${String(room.first)}`);
            if (drawn(SECTION.second, room.second) !== room.second) wrong.push(`${state}: second ${String(rows(SECTION.second))} of ${String(room.second)}`);
            if (band !== room.band) wrong.push(`${state}: band ${String(band)}`);
          }
    }
    expect(wrong).toEqual([]);
  });

  test("providers come first exactly when the city serves more models than it has providers", () => {
    const orders = [[1], [3, 1], [2, 2], [12]].map((counts) => orderOf(offersOf(city(counts))));
    expect(orders).toEqual(["models_first", "providers_first", "models_first", "providers_first"]);
  });
});

test("the level in use is the stated one when offered, else high, else the upstream's default, else none", () => {
  const offer = (levels: readonly Effort[], stated: Effort | null = null) => ({ levels: [...levels], on: "unknown" as const, from: "upstream" as const, words: [], default: stated });
  expect([
    usedLevel(offer(["low", "high"]), "low"),
    usedLevel(offer(["low", "high"]), "xhigh"),
    usedLevel(offer(["low", "medium"], "medium"), "xhigh"),
    usedLevel(offer(["low", "medium"]), null),
    usedLevel(offer([]), "high"),
  ]).toEqual([{ level: "low", by: "stated" }, { level: "high", by: "default" }, { level: "medium", by: "default" }, null, null]);
});
