// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The sessions pane as its look receives it (client/Spec.lean §7K): a
// row's state, its pin and its link, and the tag filter's toggles.

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "../../core/belief";
import { unseen } from "../../core/belief/fold";
import { MAYOR } from "../../core/route";
import type { Stretch } from "../../core/stretches";
import { PIN } from "../../core/tags";
import { Address, RunId, Seq, Tag, TimeMs } from "../../wire";
import type { SessionLine } from "../../wire";
import { dotOf, filterOf, rowOf } from "./sessions";
import type { RowContext } from "./sessions";

const LAB = Address.make("lab/room1");
const run = (doing: RunBelief["doing"]): RunBelief => ({
  ...unseen(RunId.make("00000000-0000-4000-8000-000000000001"), Seq.make(4)),
  addr: LAB,
  started: TimeMs.make(1_000),
  doing,
});
const line = (began: number): SessionLine => ({
  began: Seq.make(began),
  last: Seq.make(began + 1),
  at: TimeMs.make(5_000),
  runs: 1,
  start: { opened: { carry: "nothing", from: null } },
});
const stretch = (room: Address, current: boolean, runs: readonly RunBelief[]): Stretch => ({ room, line: line(3), current, runs });

const at = (over: Partial<RowContext>): RowContext => ({
  lang: "en",
  now: 61_000,
  chosen: false,
  tags: [],
  pinning: "none",
  named: null,
  ...over,
});

describe("a row's state dot", () => {
  test("a current session reads its last run; a past one is done whatever that run says", () => {
    const waiting = run({ kind: "waiting" });
    const thinking = run({ kind: "thinking" });
    expect([
      dotOf(stretch(LAB, true, [thinking])),
      dotOf(stretch(LAB, true, [waiting])),
      dotOf(stretch(LAB, false, [thinking])),
      dotOf(stretch(LAB, true, [])),
    ]).toEqual(["run", "ask", "done", "done"]);
  });
});

describe("a row as the look draws it", () => {
  test("the row in main says so on its link, and a past session links to itself", () => {
    const row = rowOf(stretch(LAB, false, []), at({ chosen: true }));
    expect({ current: row.link["aria-current"], href: row.link.href, chosen: row.chosen }).toEqual({
      current: "page",
      href: "#/talk/lab/room1:3",
      chosen: true,
    });
  });

  test("a pinned row names who pinned it, both ways, and draws no pin tag", () => {
    const mayor = rowOf(stretch(MAYOR, true, []), at({ pinning: "mayor" }));
    const tagged = rowOf(stretch(LAB, true, []), at({ pinning: "tagged", tags: [PIN, Tag.make("parser")] }));
    const none = rowOf(stretch(LAB, true, []), at({}));
    expect({
      mayor: mayor.pin?.wire,
      tagged: tagged.pin?.wire,
      hint: tagged.pin?.hint,
      none: none.pin,
      tags: tagged.tags,
    }).toEqual({
      mayor: { role: "img", "aria-label": "The Mayor's current session is always pinned" },
      tagged: { role: "img", "aria-label": "Pinned by you; the row's menu unpins it" },
      hint: "Pinned by you; the row's menu unpins it",
      none: undefined,
      tags: [Tag.make("parser")],
    });
  });

  test("only a current session that holds a run draws its context bar and offers the run to its menu", () => {
    const held = run({ kind: "thinking" });
    const current = rowOf(stretch(LAB, true, [held]), at({}));
    const past = rowOf(stretch(LAB, false, [held]), at({}));
    expect({
      current: [current.context, current.menu.session.run],
      past: [past.context, past.menu.session.run],
    }).toEqual({
      current: [{ room: LAB, run: held.run }, held.run],
      past: [undefined, null],
    });
  });
});

describe("the tag filter", () => {
  const words = { label: "Filter by tag", all: "all" };

  test("no tag in use draws no filter", () => {
    expect(filterOf([], null, words, () => undefined)).toBeUndefined();
  });

  test("all comes first; the held toggle is pressed, and pressing it again goes back to all", () => {
    const picked: (Tag | null)[] = [];
    const parser = Tag.make("parser");
    const filter = filterOf([parser, Tag.make("later")], parser, words, (tag) => picked.push(tag));
    filter?.toggles[1]?.wire.onclick();
    filter?.toggles[2]?.wire.onclick();
    expect({
      group: filter?.wire,
      toggles: filter?.toggles.map((toggle) => [toggle.word, toggle.wire["aria-pressed"]]),
      picked,
    }).toEqual({
      group: { role: "group", "aria-label": "Filter by tag" },
      toggles: [
        ["all", false],
        ["parser", true],
        ["later", false],
      ],
      picked: [null, Tag.make("later")],
    });
  });
});
