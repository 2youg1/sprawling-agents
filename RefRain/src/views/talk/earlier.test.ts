// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { earlierDrawn } from "./earlier";

// One accepted `/new` after a round: the current session is visibly
// empty until its first run arrives, then the earlier stretch is one
// folded entry in both readings. No history is deleted.
test("an empty new session draws no earlier run, and its first run folds the earlier stretch", () => {
  expect([
    earlierDrawn(0, 1, false),
    earlierDrawn(1, 1, false),
    earlierDrawn(2, 0, false),
    earlierDrawn(0, 0, false),
    earlierDrawn(0, 1, true),
  ]).toEqual(["none", "folded", "none", "none", "open"]);
});
