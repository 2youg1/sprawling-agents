// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { factsOf } from "./event";

test("a line keeps the fields a person reads and leaves the chain's own behind the fold", () => {
  expect(
    factsOf({
      after: 41,
      idem: "5f0c2b9e8d7a6f5e4d3c2b1a",
      prev_hash: "abc",
      task: "fix the login page",
      run: "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b",
      tokens: 1200,
      note: "",
      nested: { deep: true },
    }),
  ).toEqual([
    { name: "task", value: "fix the login page" },
    { name: "tokens", value: "1200" },
  ]);
});
