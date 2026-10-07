// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The building index's wiring as its look receives it: which entry a
// screen reader hears as current, and what a press asks for. The paint is
// measured by `cargo xtask render` on the gallery's building cases.

import { describe, expect, test } from "bun:test";

import { sectionsLookOf } from "./sections";

const SECTIONS = [
  { key: "plan", label: "Plan" },
  { key: "commits", label: "Commits" },
  { key: "skills", label: "Skills" },
] as const;

describe("the building index", () => {
  test("the section in the middle column is the one current entry", () => {
    const look = sectionsLookOf(SECTIONS, "commits", () => undefined);
    expect(look.entries.map((entry) => [entry.key, entry.current, entry.wire["aria-current"]])).toEqual([
      ["plan", false, undefined],
      ["commits", true, "true"],
      ["skills", false, undefined],
    ]);
  });

  test("while the tree's pick fills the middle column no entry is current", () => {
    const look = sectionsLookOf(SECTIONS, null, () => undefined);
    expect(look.entries.some((entry) => entry.current || entry.wire["aria-current"] !== undefined)).toBe(false);
  });

  test("a press asks for the entry's own section", () => {
    const asked: string[] = [];
    const look = sectionsLookOf(SECTIONS, "plan", (section) => {
      asked.push(section);
    });
    look.entries[2]?.wire.onclick();
    look.entries[0]?.wire.onclick();
    expect(asked).toEqual(["skills", "plan"]);
  });
});
