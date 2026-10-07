// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { cardsOf, harnessOf } from "./harnesses";

describe("harnessOf", () => {
  test("each of the three states has its word, its status and its detail", () => {
    expect([
      harnessOf({ launcher_missing: { program: "npx" } }),
      harnessOf({ not_set_up: { looked: ["~/.claude", "~/.config/claude"] } }),
      harnessOf({ ready: { at: "~/.codex" } }),
    ]).toEqual([
      { key: "harness_launcher_missing", status: "idle", said: "npx", looked: [] },
      { key: "harness_not_set_up", status: "waiting", said: null, looked: ["~/.claude", "~/.config/claude"] },
      { key: "harness_ready", status: "done", said: "~/.codex", looked: [] },
    ]);
  });
});

describe("cardsOf", () => {
  test("a known harness is named as its vendor names it, an unknown one by its id", () => {
    const cards = cardsOf(
      [
        { name: "codex", docs: "https://example.invalid/codex", launch: ["npx", "codex"], state: { ready: { at: "~/.codex" } } },
        { name: "newcomer", docs: "https://example.invalid/new", launch: ["newcomer"], state: { not_set_up: { looked: ["~/.new"] } } },
      ],
      "en",
    );
    expect(cards.map((card) => [card.name, card.launch, card.lookedIn === undefined, card.looked])).toEqual([
      ["Codex", "npx codex", true, []],
      ["newcomer", "newcomer", false, ["~/.new"]],
    ]);
  });
});
