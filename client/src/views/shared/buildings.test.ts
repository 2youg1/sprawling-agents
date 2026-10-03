// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { Answer } from "../../wire";
import { Address, Tokens, UsdMicros } from "../../wire";
import { HALL, buildingsOf } from "./buildings";

const IDLE = { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 } };

function cityOf(...addrs: readonly string[]): Answer {
  return {
    city: {
      active: 0,
      buildings: addrs.map((addr) => ({ addr: Address.make(addr), blocked: [], problems: [], progress: IDLE, ready: 0 })),
      frozen: 0,
      halted: [],
      pursuits: [],
      runs: [],
    },
  };
}

describe("buildingsOf", () => {
  // The User's report: a city with one building gave the rules page
  // nothing to pick. The picker lists the hall and that building.
  test("a city with one building offers the hall and that building", () => {
    expect(buildingsOf(cityOf("kiln"))).toEqual([HALL, Address.make("kiln")]);
  });

  test("the hall comes first once, whether or not the city lists it", () => {
    expect(buildingsOf(cityOf("lab", HALL, "atelier"))).toEqual([HALL, Address.make("atelier"), Address.make("lab")]);
  });

  test("a city that has not answered is the hall alone, never an empty list", () => {
    expect(buildingsOf(undefined)).toEqual([HALL]);
  });
});
