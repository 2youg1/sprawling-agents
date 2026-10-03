// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { earlierDrawn } from "./earlier";

// The regression of roadmap A23: one round, then a model change opened a
// new session, so the round became the earlier stretch. "Everything"
// drew one folded line over an empty session while "results only" drew
// the round open; both modes now read this one answer.
test("the earlier stretch opens while the open session holds no run, and folds once it does", () => {
  expect([earlierDrawn(0, 1), earlierDrawn(1, 1), earlierDrawn(2, 0), earlierDrawn(0, 0)]).toEqual([
    "open",
    "folded",
    "none",
    "none",
  ]);
});
