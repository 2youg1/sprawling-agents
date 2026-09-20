// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the control decides before anything is drawn: where a track
// begins and ends, where an arrow key lands, and which cell a keyboard
// arrives at. The drawing itself is measured in a real engine by
// `cargo xtask render` against the gallery fixtures, because where a
// box landed is not something a unit test can be told.

import { describe, expect, test } from "bun:test";

import { bands, nextStop, tabStop, type Choice, type Group } from "./segmented";

type Api = "chat" | "responses" | "messages";

const OPEN_AI: Group = { label: "OpenAI", tone: "accent" };
const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

// The three dialects as the provider form offers them: two from one
// lab, one from another, and the middle one not yet carried.
const CHAT: Choice<Api> = { value: "chat", label: "chat", group: OPEN_AI };
const RESPONSES: Choice<Api> = {
  value: "responses",
  label: "responses",
  group: OPEN_AI,
  why: "next version",
};
const MESSAGES: Choice<Api> = { value: "messages", label: "messages", group: ANTHROPIC };
const DIALECTS: readonly Choice<Api>[] = [CHAT, RESPONSES, MESSAGES];

// The same three values with nothing said about where they come from.
const PLAIN: readonly Choice<Api>[] = [
  { value: "chat", label: "chat" },
  { value: "responses", label: "responses" },
  { value: "messages", label: "messages" },
];

const REFUSED: readonly Choice<Api>[] = PLAIN.map((choice) => ({ ...choice, why: "next version" }));

describe("a flat list is cut into the tracks that are drawn", () => {
  test("cells that state no group are one track", () => {
    expect(bands(PLAIN)).toEqual([{ group: undefined, from: 0, cells: PLAIN }]);
  });

  test("a change of group opens a track and records where it starts", () => {
    expect(bands(DIALECTS)).toEqual([
      { group: OPEN_AI, from: 0, cells: [CHAT, RESPONSES] },
      { group: ANTHROPIC, from: 2, cells: [MESSAGES] },
    ]);
  });

  test("one name under two tones draws two tracks rather than one of them", () => {
    const split: readonly Choice<Api>[] = [
      { value: "chat", label: "chat", group: { label: "OpenAI", tone: "accent" } },
      { value: "responses", label: "responses", group: { label: "OpenAI", tone: "alert" } },
    ];
    expect(bands(split).length).toBe(2);
  });

  test("the tracks together are the list the caller gave, in order", () => {
    expect(bands(DIALECTS).flatMap((band) => band.cells)).toEqual([...DIALECTS]);
  });

  test("an empty control draws no track", () => {
    expect(bands<Api>([])).toEqual([]);
  });
});

describe("an arrow key crosses the whole control", () => {
  test("the right arrow wraps past the last cell to the first", () => {
    expect(nextStop(PLAIN, 2, 1)).toBe(0);
  });

  test("the left arrow wraps past the first cell to the last", () => {
    expect(nextStop(PLAIN, 0, -1)).toBe(2);
  });

  test("a cell that cannot be chosen is stepped over, not landed on", () => {
    expect(nextStop(DIALECTS, 0, 1)).toBe(2);
    expect(nextStop(DIALECTS, 2, -1)).toBe(0);
  });

  test("a control whose every cell is refused stays where it is", () => {
    expect(nextStop(REFUSED, 1, 1)).toBe(1);
  });

  test("a control with one cell answers that cell", () => {
    expect(nextStop([CHAT], 0, 1)).toBe(0);
  });
});

describe("a keyboard reaches the control in one stop", () => {
  test("the stop is the chosen cell", () => {
    expect(tabStop(DIALECTS, "messages")).toBe(2);
  });

  test("a held value no cell carries falls to the first choosable cell", () => {
    expect(tabStop([RESPONSES, MESSAGES], "chat")).toBe(1);
  });

  test("a control where every cell is refused still offers its reason", () => {
    expect(tabStop<string>(REFUSED, "nothing")).toBe(0);
  });

  test("an empty control offers nothing", () => {
    expect(tabStop<Api>([], "chat")).toBe(-1);
  });
});
