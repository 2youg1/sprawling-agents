// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { standing } from "./standing";
import type { Answer } from "../wire";

const building = (answer: Answer): string | undefined => ("config" in answer ? "picked" : undefined);

describe("an answer the city cannot give is not a pending one", () => {
  test("unavailable reads as unavailable", () => {
    const held: Answer = { unavailable: { query: "BuildingView(shop)" } };
    expect(standing(held, building)).toEqual({ kind: "unavailable" });
  });

  test("nothing held yet reads as asking", () => {
    expect(standing(undefined, building)).toEqual({ kind: "asking" });
  });
});
