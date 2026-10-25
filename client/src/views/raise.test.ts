// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a typed address would become on the raise page, read against the
// city's own list of buildings and the folders waiting under it.

import { describe, expect, test } from "bun:test";

import { Address } from "../wire";
import type { BuildingProgress } from "../wire";
import { fillable, raisingOf, waiting } from "./raise";

const IDLE: BuildingProgress["progress"] = { planned: { blocked: 0, blocked_ppb: 0, done: 0, done_ppb: 0, total: 0 } };

function line(addr: string, adoptable: boolean): BuildingProgress {
  return { addr: Address.make(addr), adoptable, blocked: [], problems: [], progress: IDLE, ready: 0 };
}

const CITY = [line("lab", false), line("notes", true)];

describe("the raise page", () => {
  test("a typed address is built, adopted or refused by what the city lists, and nothing typed is nothing yet", () => {
    expect(["", "shop", "notes", "lab", "lab/parser", "/lab"].map((typed) => raisingOf(typed, CITY))).toEqual([
      { kind: "blank" },
      { kind: "build", addr: Address.make("shop") },
      { kind: "adopt", addr: Address.make("notes") },
      { kind: "refused", why: "raise_taken" },
      { kind: "refused", why: "raise_room" },
      { kind: "refused", why: "raise_unreadable" },
    ]);
  });

  test("the folders waiting to be adopted are the ones the city says are, and the one in the box is not offered", () => {
    expect([waiting(CITY), fillable(CITY, ""), fillable(CITY, "notes")]).toEqual([[Address.make("notes")], [Address.make("notes")], []]);
  });
});
