// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The palette's list as the listbox its box controls: sections named
// by their headings, a verb this place cannot run kept and greyed with
// its reason (ux 7-8), a page's row ending in its key's chord, and the
// pointer and the click reported against the flat list the cursor walks.

import { describe, expect, test } from "bun:test";

import { say } from "../../core/lang";
import type { Entry } from "./entry";
import { rowsLookOf } from "./rows";

function entry(label: string, more: Partial<Entry> = {}): Entry {
  return { label, hint: `${label} hint`, act: () => undefined, ...more };
}

describe("the palette's list", () => {
  const halt = entry("/halt");
  const stop = entry("/stop", { why: "no_run_in_front" });
  const fresh = entry("/new");
  const shown = [halt, stop, fresh];
  const verbs = rowsLookOf(
    {
      listing: {
        kind: "verbs",
        groups: [
          { section: "actions", entries: [halt, stop] },
          { section: "sessions", entries: [fresh] },
        ],
      },
      shown,
      cursor: 1,
      id: "l",
      lang: "en",
    },
    { hover: () => undefined, pick: () => undefined },
  );

  test("a section is a group named by its own heading", () => {
    expect(
      verbs.sections.map((section) => [section.wire, section.heading?.wire.id, section.heading?.text]),
    ).toEqual([
      [{ role: "group", "aria-labelledby": "l-actions" }, "l-actions", say("en", "palette_group_actions")],
      [{ role: "group", "aria-labelledby": "l-sessions" }, "l-sessions", say("en", "palette_group_sessions")],
    ]);
  });

  test("a row's id is its place in the flat list, and the cursor may stand on a refused verb", () => {
    const rows = verbs.sections.flatMap((section) => section.rows);
    expect(
      rows.map((row) => [row.wire.id, row.wire["aria-selected"], row.wire["aria-disabled"], row.active, row.refused]),
    ).toEqual([
      ["l-0", false, false, false, false],
      ["l-1", true, true, true, true],
      ["l-2", false, false, false, false],
    ]);
  });

  test("a refused verb says why where its purpose would be", () => {
    const rows = verbs.sections.flatMap((section) => section.rows);
    expect(rows.map((row) => row.end)).toEqual([
      { kind: "hint", text: "/halt hint" },
      { kind: "hint", text: say("en", "no_run_in_front") },
      { kind: "hint", text: "/new hint" },
    ]);
  });

  test("the pointer and the click are reported against the flat list", () => {
    const hovered: number[] = [];
    const picked: Entry[] = [];
    const look = rowsLookOf(
      { listing: { kind: "places" }, shown: [halt, entry("city", { action: "go.city" })], cursor: 0, id: "l", lang: "en" },
      { hover: (at) => hovered.push(at), pick: (each) => picked.push(each) },
    );
    const [section] = look.sections;
    const second = section?.rows[1];
    second?.wire.onmouseenter();
    second?.wire.onclick();
    expect(hovered).toEqual([1]);
    expect(picked.map((each) => each.label)).toEqual(["city"]);
    expect(section?.heading).toBeUndefined();
    expect(second?.end).toEqual({ kind: "chord", action: "go.city" });
  });
});
