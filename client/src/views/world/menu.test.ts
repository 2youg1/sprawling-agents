// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The world layer's menu buttons as their looks receive them: the key
// table of client/spec/Views/Workspace.lean §7-11 (pane label, session
// row menu), and the `aria-*` values and items the wire bags carry.

import { describe, expect, test } from "bun:test";

import { WORKBENCH } from "../../core/workbench";
import type { Side } from "../../core/workbench";
import { opening, walked } from "./menu";
import { paneMenuLookOf } from "./pane_menu";
import type { PaneMenuHands } from "./pane_menu";
import { sessionMenuLookOf } from "./session_menu";
import type { SessionMenuHands } from "./session_menu";

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

describe("walking an open menu", () => {
  test("Up and Down wrap at both ends", () => {
    expect(walked(3, 2, "ArrowDown")).toEqual({ kind: "focus", at: 0 });
    expect(walked(3, 0, "ArrowUp")).toEqual({ kind: "focus", at: 2 });
    expect(walked(3, 1, "ArrowDown")).toEqual({ kind: "focus", at: 2 });
  });

  test("from no item, Down lands on the first and Up on the last", () => {
    expect(walked(3, -1, "ArrowDown")).toEqual({ kind: "focus", at: 0 });
    expect(walked(3, -1, "ArrowUp")).toEqual({ kind: "focus", at: 2 });
  });

  test("Escape closes, Tab lets the menu go, and an empty menu moves nowhere", () => {
    expect(walked(3, 1, "Escape")).toEqual({ kind: "close" });
    expect(walked(3, 1, "Tab")).toEqual({ kind: "leave" });
    expect(walked(0, -1, "ArrowDown")).toEqual({ kind: "none" });
    expect(walked(3, 1, "a")).toEqual({ kind: "none" });
  });

  test("Down on a closed button opens the menu, and on an open one does nothing", () => {
    const shown: string[] = [];
    const closed = press("ArrowDown");
    opening(false, { show: () => shown.push("show") })(closed);
    const open = press("ArrowDown");
    opening(true, { show: () => shown.push("show") })(open);
    expect({ shown, closed: closed.prevented(), open: open.prevented() }).toEqual({ shown: ["show"], closed: true, open: false });
  });
});

function paneHands(log: string[]): PaneMenuHands {
  return {
    live: () => [],
    show: () => log.push("show"),
    close: () => log.push("close"),
    leave: () => log.push("leave"),
    move: (side: Side) => log.push(`move ${side}`),
    hold: () => () => undefined,
  };
}

describe("a pane label's menu", () => {
  const words = { label: "sessions", arrange: "arrange sessions", left: "move left", right: "move right" };

  test("the pane on the left edge cannot move further left, and the item says so by being disabled", () => {
    const look = paneMenuLookOf({ pane: "sessions", bench: WORKBENCH, arranged: "menu", open: true, uid: "p" }, words, paneHands([]));
    expect(look.items.map((item) => [item.key, item.wire.disabled])).toEqual([
      ["left", true],
      ["right", false],
    ]);
  });

  test("the button names the menu it controls and says whether it is open", () => {
    const look = paneMenuLookOf({ pane: "session", bench: WORKBENCH, arranged: "menu", open: false, uid: "p" }, words, paneHands([]));
    expect({
      haspopup: look.trigger["aria-haspopup"],
      expanded: look.trigger["aria-expanded"],
      controls: look.trigger["aria-controls"],
      popup: look.popup.id,
      name: look.trigger["aria-label"],
      role: look.list.role,
      holder: look.items[0]?.holder.role,
      item: look.items[0]?.wire.role,
    }).toEqual({
      haspopup: "menu",
      expanded: false,
      controls: "p-menu",
      popup: "p-menu",
      name: "arrange sessions",
      role: "menu",
      holder: "none",
      item: "menuitem",
    });
  });

  test("a press on the button opens a closed menu and closes an open one; an item moves the pane", () => {
    const log: string[] = [];
    paneMenuLookOf({ pane: "session", bench: WORKBENCH, arranged: "menu", open: false, uid: "p" }, words, paneHands(log)).trigger.onclick();
    const open = paneMenuLookOf({ pane: "session", bench: WORKBENCH, arranged: "menu", open: true, uid: "p" }, words, paneHands(log));
    open.trigger.onclick();
    open.items[1]?.wire.onclick();
    expect(log).toEqual(["show", "close", "move right"]);
  });
});

function sessionHands(log: string[]): SessionMenuHands {
  return {
    live: () => [],
    show: () => log.push("show"),
    close: () => log.push("close"),
    leave: () => log.push("leave"),
    type: (typed: string) => log.push(`type ${typed}`),
    submit: () => log.push("submit"),
    hold: () => () => undefined,
  };
}

describe("a session row's menu", () => {
  const items = [{ id: "pin", word: "pin", act: () => undefined }];

  test("a city with no name leaves the button disabled, with the reason beside it", () => {
    const look = sessionMenuLookOf(
      { uid: "s", open: false, why: "not connected", name: "Actions for room1", items, naming: null },
      sessionHands([]),
    );
    expect({ disabled: look.trigger.disabled, why: look.why }).toEqual({ disabled: true, why: "not connected" });
  });

  test("the field the menu turns into is described by its hint and marked invalid with it", () => {
    const look = sessionMenuLookOf(
      {
        uid: "s",
        open: true,
        why: undefined,
        name: "Actions for room1",
        items,
        naming: { label: "Tag", hint: "One word", typed: "two words", wrong: true, max: 24 },
      },
      sessionHands([]),
    );
    expect({
      described: look.naming?.field["aria-describedby"],
      hint: look.naming?.hint.id,
      invalid: look.naming?.field["aria-invalid"],
      max: look.naming?.field.maxlength,
    }).toEqual({ described: "s-hint", hint: "s-hint", invalid: true, max: 24 });
  });

  test("Escape in the field closes the menu, and any other key is the field's", () => {
    const log: string[] = [];
    const look = sessionMenuLookOf(
      {
        uid: "s",
        open: true,
        why: undefined,
        name: "Actions for room1",
        items,
        naming: { label: "Tag", hint: "One word", typed: "", wrong: false, max: 24 },
      },
      sessionHands(log),
    );
    const escape = press("Escape");
    const letter = press("a");
    look.naming?.field.onkeydown(escape);
    look.naming?.field.onkeydown(letter);
    expect({ log, escape: escape.prevented(), letter: letter.prevented() }).toEqual({ log: ["close"], escape: true, letter: false });
  });
});
