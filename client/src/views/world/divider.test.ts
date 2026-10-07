// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A workbench divider and the head of the world's sheet as their looks
// receive them: the APG Window Splitter and Tabs key tables of
// client/spec/Views/Workspace.lean §7-11, and the `aria-*` values their
// bags carry. The clamping of a width is `core/workbench.ts`'s, judged
// by its own test.

import { describe, expect, test } from "bun:test";

import { NARROWEST } from "../../core/workbench";
import type { Pane } from "../../core/workbench";
import { dividerLookOf, stepOf } from "./divider";
import type { DividerHands } from "./divider";
import { landing, sheetHeadOf } from "./sheet_head";

const press = (key: string): { key: string; preventDefault: () => void; prevented: () => boolean } => {
  let prevented = false;
  return {
    key,
    preventDefault: () => {
      prevented = true;
    },
    prevented: () => prevented,
  };
};

describe("a divider's keys", () => {
  test("the arrows move one column, Home and End to the ends, Enter resets", () => {
    expect([
      stepOf("ArrowLeft", 3, 7),
      stepOf("ArrowRight", 3, 7),
      stepOf("Home", 3, 7),
      stepOf("End", 3, 7),
      stepOf("Enter", 3, 7),
      stepOf("ArrowUp", 3, 7),
    ]).toEqual([
      { kind: "span", span: 2 },
      { kind: "span", span: 4 },
      { kind: "span", span: NARROWEST },
      { kind: "span", span: 7 },
      { kind: "reset" },
      { kind: "none" },
    ]);
  });

  test("a key the splitter takes is kept from the page, and one it does not take is left alone", () => {
    const log: string[] = [];
    const hands: DividerHands = {
      set: (span) => log.push(`set ${String(span)}`),
      reset: () => log.push("reset"),
      drag: (now) => log.push(`drag ${String(now)}`),
      follow: (x) => log.push(`follow ${String(x)}`),
    };
    const look = dividerLookOf({ label: "width of sessions", controls: "w-sessions", span: 3, widest: 7, dragging: false }, hands);
    const end = press("End");
    const enter = press("Enter");
    const tab = press("Tab");
    look.wire.onkeydown(end);
    look.wire.onkeydown(enter);
    look.wire.onkeydown(tab);
    expect({ log, prevented: [end.prevented(), enter.prevented(), tab.prevented()] }).toEqual({
      log: ["set 7", "reset"],
      prevented: [true, true, false],
    });
  });

  test("the separator says what it sizes and how wide that is, in columns", () => {
    const hands: DividerHands = { set: () => undefined, reset: () => undefined, drag: () => undefined, follow: () => undefined };
    const { wire } = dividerLookOf({ label: "width of sessions", controls: "w-sessions", span: 3, widest: 7, dragging: false }, hands);
    expect([wire.role, wire.tabindex, wire["aria-orientation"], wire["aria-controls"], wire["aria-valuenow"], wire["aria-valuemin"], wire["aria-valuemax"]]).toEqual([
      "separator",
      0,
      "vertical",
      "w-sessions",
      3,
      NARROWEST,
      7,
    ]);
  });
});

describe("the world sheet's tabs", () => {
  test("the arrows wrap at both ends, Home and End go to the first and the last", () => {
    expect([landing(3, 2, "ArrowRight"), landing(3, 0, "ArrowLeft"), landing(3, 1, "Home"), landing(3, 1, "End"), landing(3, 1, "Enter"), landing(0, -1, "Home")]).toEqual([
      0,
      2,
      0,
      2,
      undefined,
      undefined,
    ]);
  });

  test("only the shown tab is a stop; each controls its pane and is the label of it", () => {
    const shown: Pane[] = [];
    const focused: Pane[] = [];
    const look = sheetHeadOf(
      {
        tabs: [
          { pane: "sessions", label: "sessions" },
          { pane: "session", label: "the chosen session" },
          { pane: "commits", label: "lab" },
        ],
        shown: "commits",
        prefix: "w",
        words: { back: "back to the conversation", strip: "what the world layer shows" },
      },
      { leave: () => undefined, show: (pane) => shown.push(pane), focus: (pane) => focused.push(pane), hold: () => () => undefined },
    );
    const right = press("ArrowRight");
    look.tabs[2]?.wire.onkeydown(right);
    expect({
      stops: look.tabs.map((tab) => tab.wire.tabindex),
      ids: look.tabs.map((tab) => [tab.wire.id, tab.wire["aria-controls"], tab.wire["aria-selected"]]),
      shown,
      focused,
      prevented: right.prevented(),
    }).toEqual({
      stops: [-1, -1, 0],
      ids: [
        ["w-tab-sessions", "w-sessions", false],
        ["w-tab-session", "w-session", false],
        ["w-tab-commits", "w-commits", true],
      ],
      shown: ["sessions"],
      focused: ["sessions"],
      prevented: true,
    });
  });
});
