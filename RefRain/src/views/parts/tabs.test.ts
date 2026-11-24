// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { landing, panelId, panelStop, tabId, tabStop } from "./tab_navigation";

const SIZES = [1, 2, 3, 4, 5, 6, 7, 8];
const positions = (total: number): readonly number[] =>
  Array.from({ length: total }, (_, at) => at);

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

describe("the browser's tab stops and labels", () => {
  const IDs = ["time", "turns", "changes"];

  test("only the current tab is a Tab stop", () => {
    expect(IDs.map((_, index) => tabStop(index, 1))).toEqual([-1, 0, -1]);
  });

  test("a current id that names no lens leaves the first tab as the way in", () => {
    expect(IDs.map((_, index) => tabStop(index, -1))).toEqual([0, -1, -1]);
  });

  test("tabs and panels within a group have unique, predictable labels", () => {
    const group = "run";
    const tabIds = IDs.map((id) => tabId(group, id));
    const panelIds = IDs.map((id) => panelId(group, id));
    expect(tabIds[1]).toBe("run-tab-turns");
    expect(panelIds[1]).toBe("run-panel-turns");
    expect(new Set([...tabIds, ...panelIds]).size).toBe(IDs.length * 2);
  });

  test("two groups on one page never claim each other's panel ids", () => {
    const one = new Set(IDs.map((id) => panelId("run", id)));
    expect(IDs.every((id) => !one.has(panelId("record", id)))).toBe(true);
  });

  test("the shown panel is a tab stop while nothing inside can hold focus", () => {
    expect([panelStop(true, false), panelStop(true, true), panelStop(false, false)]).toEqual([
      0,
      undefined,
      undefined,
    ]);
  });
});
