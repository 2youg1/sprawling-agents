// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tabs' wiring, driven the way its seat drives it and with no look
// at all: whichever look draws the tabs, these are the keys it hands on,
// the one Tab stop, and the ids that tie each tab to its panel
// (client/spec/Views/Parts.lean §7-4). The key walk is checked over
// every position of every set up to eight tabs, the space the model in
// client/spec/Views/Parts/Tabs.lean quantifies over.

import { describe, expect, test } from "bun:test";
import { createRawSnippet } from "svelte";
import type { Attachment } from "svelte/attachments";

import { landing, lookOf } from "./tabs";
import type { Hands, Lens, Pressed, TabsLook } from "./tabs";

const SIZES = [1, 2, 3, 4, 5, 6, 7, 8];
const positions = (total: number): readonly number[] => Array.from({ length: total }, (_, at) => at);

describe("a key lands inside the set, as the model walks it", () => {
  test("every arrow, Home and End lands on a tab of the set", () => {
    for (const total of SIZES) {
      for (const at of positions(total)) {
        for (const key of ["ArrowRight", "ArrowLeft", "Home", "End"]) {
          const to = landing(total, at, key);
          expect(to ?? -1).toBeGreaterThanOrEqual(0);
          expect(to ?? total).toBeLessThan(total);
        }
      }
    }
  });

  test("the left arrow undoes the right one, so both wrap at the ends", () => {
    for (const total of SIZES) {
      for (const at of positions(total)) {
        expect(landing(total, landing(total, at, "ArrowRight") ?? -1, "ArrowLeft")).toBe(at);
      }
    }
  });

  test("Home and End reach the ends from anywhere", () => {
    for (const total of SIZES) {
      for (const at of positions(total)) {
        expect([landing(total, at, "Home"), landing(total, at, "End")]).toEqual([0, total - 1]);
      }
    }
  });

  test("Space, Enter and every other key are left to the click path", () => {
    for (const key of [" ", "Enter", "ArrowDown", "a"]) {
      expect(landing(3, 1, key)).toBeUndefined();
    }
  });
});

const LENSES: readonly Lens[] = [
  { id: "time", label: "time" },
  { id: "turns", label: "turns" },
  { id: "changes", label: "changes" },
];

// The wiring hands the caller's snippet on without calling it.
const PANEL = createRawSnippet<[Lens]>(() => ({ render: () => "<p></p>" }));

// One set of tabs and everything around it, recorded.
function rig(current: string, reachable = true) {
  const picked: string[] = [];
  const focused: string[] = [];
  const kept = new Map<string, Attachment<HTMLElement>>();
  const watch: Attachment<HTMLElement> = () => undefined;
  const hands: Hands = {
    focus: (id) => focused.push(id),
    keep: (id) => {
      const made = kept.get(id) ?? (() => undefined);
      kept.set(id, made);
      return made;
    },
    watch,
    reachable,
  };
  const look: TabsLook = lookOf(
    { label: "lenses", lenses: LENSES, current, onPick: (id) => picked.push(id), panel: PANEL },
    "u1",
    hands,
  );
  return { look, picked, focused, kept, watch };
}

function press(key: string): Pressed & { readonly prevented: () => boolean } {
  let prevented = false;
  return {
    key,
    preventDefault: () => {
      prevented = true;
    },
    prevented: () => prevented,
  };
}

const symbols = (bag: object): readonly symbol[] => Object.getOwnPropertySymbols(bag);

describe("the wire bags a look spreads", () => {
  test("only the current tab is a Tab stop, and only it is selected", () => {
    const { look } = rig("turns");
    expect(look.tabs.map((tab) => [tab.wire.tabindex, tab.wire["aria-selected"], tab.current])).toEqual([
      [-1, false, false],
      [0, true, true],
      [-1, false, false],
    ]);
  });

  test("a current id that names no lens leaves the first tab as the way in", () => {
    const { look } = rig("gone");
    expect(look.tabs.map((tab) => tab.wire.tabindex)).toEqual([0, -1, -1]);
    expect(look.tabs.some((tab) => tab.wire["aria-selected"])).toBe(false);
  });

  test("each tab controls its own panel and each panel is labelled by its own tab", () => {
    const { look } = rig("time");
    expect(look.list).toEqual({ role: "tablist", "aria-label": "lenses" });
    look.tabs.forEach((tab, at) => {
      const panel = look.panels[at];
      expect(tab.wire.role).toBe("tab");
      expect(panel?.wire.role).toBe("tabpanel");
      expect(tab.wire["aria-controls"]).toBe(panel?.wire.id ?? "");
      expect(panel?.wire["aria-labelledby"]).toBe(tab.wire.id);
    });
    expect(new Set(look.tabs.map((tab) => tab.wire.id)).size).toBe(LENSES.length);
  });

  test("two sets on one page never claim each other's panels", () => {
    const first = rig("time").look;
    const second = lookOf(
      { label: "lenses", lenses: LENSES, current: "time", onPick: () => undefined, panel: PANEL },
      "u2",
      { focus: () => undefined, keep: () => () => undefined, watch: () => undefined, reachable: true },
    );
    const ids = (look: TabsLook): readonly string[] => [
      ...look.tabs.map((tab) => tab.wire.id),
      ...look.panels.map((panel) => panel.wire.id),
    ];
    expect(ids(first).filter((id) => ids(second).includes(id))).toEqual([]);
  });

  test("an arrow picks the next lens, moves the focus there and keeps the key", () => {
    const { look, picked, focused } = rig("changes");
    const key = press("ArrowRight");
    look.tabs[2]?.wire.onkeydown(key);
    expect([picked, focused, key.prevented()]).toEqual([["time"], ["time"], true]);
  });

  test("a key the tabs leave alone picks nothing and keeps nothing", () => {
    const { look, picked, focused } = rig("turns");
    const key = press(" ");
    look.tabs[1]?.wire.onkeydown(key);
    expect([picked, focused, key.prevented()]).toEqual([[], [], false]);
  });

  test("a click picks its lens and leaves the focus where the click put it", () => {
    const { look, picked, focused } = rig("time");
    look.tabs[1]?.wire.onclick();
    expect([picked, focused]).toEqual([["turns"], []]);
  });

  test("each tab carries the seat's hold for its own lens", () => {
    const { look, kept } = rig("time");
    look.tabs.forEach((tab) => {
      const [hold] = symbols(tab.wire);
      expect(hold === undefined ? undefined : tab.wire[hold]).toBe(kept.get(tab.lens.id));
    });
  });
});

describe("the panels", () => {
  test("only the shown panel is drawn; the others are hidden wrappers", () => {
    const { look } = rig("turns");
    expect(look.panels.map((panel) => [panel.shown, panel.wire.hidden])).toEqual([
      [false, true],
      [true, false],
      [false, true],
    ]);
  });

  test("the shown panel is a stop of its own exactly while nothing inside takes the focus", () => {
    expect(rig("turns", false).look.panels.map((panel) => panel.wire.tabindex)).toEqual([undefined, 0, undefined]);
    expect(rig("turns", true).look.panels.map((panel) => panel.wire.tabindex)).toEqual([
      undefined,
      undefined,
      undefined,
    ]);
  });

  test("the seat watches the shown panel and no other", () => {
    const { look, watch } = rig("turns");
    expect(look.panels.map((panel) => symbols(panel.wire).map((key) => panel.wire[key]))).toEqual([[], [watch], []]);
  });
});
