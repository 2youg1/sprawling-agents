// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { AxError } from "../../wire";
import { pressable, type Standing, standingOf } from "./card";
import { HELD, sent, waited } from "./saving";

const words = (key: string): string => key;

const refusal: AxError = {
  action: "replace a document",
  code: "E_VERSION_CONFLICT",
  nearby: [],
  recovery: "read the file again",
  retry: "no",
  subject: "MAYOR.md",
};

describe("a settings card's foot", () => {
  test("says when a saved change takes effect, on every card", () => {
    const saved: Standing = { word: "saving_saved", weight: "quiet", saved: true, detail: "next run" };
    expect(standingOf({ kind: "saved" }, "next run", true, words)).toEqual(saved);
    expect(standingOf({ kind: "saved" }, "next run", false, words)).toEqual(saved);
  });

  test("on a card saved by its picks, says it before anything is saved", () => {
    expect(standingOf(HELD, "next run", false, words)).toEqual({ word: null, weight: "quiet", saved: false, detail: "next run" });
    expect(standingOf(HELD, "next run", true, words)).toEqual({ word: null, weight: "quiet", saved: false, detail: null });
  });

  test("gives the city's way on after a refusal", () => {
    expect(standingOf({ kind: "refused", error: refusal }, "next run", true, words)).toEqual({
      word: "saving_refused",
      weight: "alert",
      saved: false,
      detail: "read the file again",
    });
  });

  test("lets the save be pressed again only when there is something to send", () => {
    const states = [HELD, { kind: "draft" } as const, sent("v1"), waited(sent("v1")), { kind: "saved" } as const];
    expect(states.map(pressable)).toEqual([false, true, false, true, false]);
  });
});
