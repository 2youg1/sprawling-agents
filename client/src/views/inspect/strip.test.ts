// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The inspector strip's key table and `aria-*` values (client/Spec.lean
// §7-11, the APG Tabs rows), read off the value the seat hands any look
// (client D95). No look is imported here, so a look swapped for another
// leaves every case standing.

import { describe, expect, test } from "bun:test";

import { landing, lookOf, tabStop, type StripHands, type StripLook, type StripTab } from "./strip";

function tab(key: string, front: boolean): StripTab {
  return { key, label: key, terminal: false, unsaved: false, front, href: `#/talk/${key}`, controls: "inspector-editor" };
}

const WORDS = {
  tabs: "open in the inspector",
  unsaved: "unsaved",
  closeAll: "close the inspector",
  closeItem: (name: string) => `close ${name}`,
};

interface Heard {
  readonly picked: number[];
  readonly closed: number[];
  readonly focused: number[];
  closedAll: number;
}

function drawn(tabs: readonly StripTab[]): { readonly look: StripLook; readonly heard: Heard } {
  const heard: Heard = { picked: [], closed: [], focused: [], closedAll: 0 };
  const hands: StripHands = {
    pick: (at) => heard.picked.push(at),
    close: (at) => heard.closed.push(at),
    closeAll: () => {
      heard.closedAll += 1;
    },
    focus: (at) => heard.focused.push(at),
    hold: () => () => undefined,
  };
  return { look: lookOf(tabs, WORDS, hands), heard };
}

function press(look: StripLook, at: number, key: string): boolean {
  let prevented = false;
  look.tabs[at]?.wire.onkeydown({
    key,
    preventDefault: () => {
      prevented = true;
    },
  });
  return prevented;
}

const THREE = [tab("a", false), tab("b", true), tab("c", false)];

describe("the strip is one Tab stop", () => {
  test("the front tab holds it", () => {
    const { look } = drawn(THREE);
    expect(look.tabs.map((each) => each.wire.tabindex)).toEqual([-1, 0, -1]);
    expect(tabStop(THREE)).toBe(1);
  });

  test("the first tab holds it when none is in front", () => {
    const { look } = drawn(THREE.map((each) => ({ ...each, front: false })));
    expect(look.tabs.map((each) => each.wire.tabindex)).toEqual([0, -1, -1]);
  });

  test("a tab's own close mark is never a stop", () => {
    const { look } = drawn(THREE);
    expect(look.tabs.map((each) => each.close.tabindex)).toEqual([-1, -1, -1]);
  });
});

describe("the arrows walk the strip and bring each tab forward", () => {
  test("they wrap at either end, and Home and End go to the ends", () => {
    expect([landing(3, 0, "ArrowLeft"), landing(3, 2, "ArrowRight"), landing(3, 1, "Home"), landing(3, 1, "End")]).toEqual([
      2, 0, 0, 2,
    ]);
    expect(landing(3, 1, "a")).toBeUndefined();
  });

  test("a landing picks the tab there and moves the focus to it", () => {
    const { look, heard } = drawn(THREE);
    expect(press(look, 1, "ArrowRight")).toBe(true);
    expect(heard).toEqual({ picked: [2], closed: [], focused: [2], closedAll: 0 });
  });

  test("a key the strip does not take is left to the page", () => {
    const { look, heard } = drawn(THREE);
    expect(press(look, 1, "Enter")).toBe(false);
    expect(heard).toEqual({ picked: [], closed: [], focused: [], closedAll: 0 });
  });
});

describe("Delete closes the tab under the focus", () => {
  test("the focus stays at that place, or on the new last tab", () => {
    const middle = drawn(THREE);
    expect(press(middle.look, 1, "Delete")).toBe(true);
    expect(middle.heard).toEqual({ picked: [], closed: [1], focused: [1], closedAll: 0 });
    const last = drawn(THREE);
    press(last.look, 2, "Delete");
    expect(last.heard.focused).toEqual([1]);
  });
});

describe("the bags carry the Tabs pattern's values", () => {
  test("a tab names the panel it controls and whether it is selected, and links to its item", () => {
    const { look } = drawn([{ ...tab("a", true), controls: "inspector-terminal", unsaved: true, terminal: true }]);
    const [only] = look.tabs;
    expect(only?.wire.role).toBe("tab");
    expect(only?.wire["aria-selected"]).toBe(true);
    expect(only?.wire["aria-controls"]).toBe("inspector-terminal");
    expect(only?.wire.href).toBe("#/talk/a");
    expect(only?.unsaved).toBe("unsaved");
    expect(only?.close["aria-label"]).toBe("close a");
    expect(look.list).toEqual({ role: "tablist", "aria-label": "open in the inspector" });
  });

  test("an item the address bar cannot spell has no href", () => {
    const { look } = drawn([{ ...tab("a", true), href: null }]);
    expect(look.tabs[0]?.wire.href).toBeUndefined();
  });

  test("a click brings the tab forward without leaving the page", () => {
    const { look, heard } = drawn(THREE);
    let prevented = false;
    look.tabs[0]?.wire.onclick({
      preventDefault: () => {
        prevented = true;
      },
    });
    look.tabs[2]?.close.onclick();
    look.closeAll.onclick();
    expect(prevented).toBe(true);
    expect(heard).toEqual({ picked: [0], closed: [2], focused: [], closedAll: 1 });
  });
});
