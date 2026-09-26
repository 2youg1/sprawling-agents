// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { recoveryFor } from "./recovering";

describe("the recovery table", () => {
  // A page and a city that disagree about the wire meet the same
  // disagreement on every reconnect: offering "try again" there ran the
  // person round a loop, and fetching the client this city was built
  // with is the one thing that ends it.
  test("a wire mismatch offers a reload, never a reconnect", () => {
    expect(recoveryFor("E_WIRE_MISMATCH").map((recovery) => recovery.kind)).toEqual(["reload"]);
  });
});
