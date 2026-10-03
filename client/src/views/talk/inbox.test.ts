// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The queued-signal section names every kind `kernel::SignalKind` spells
// and shows an unknown one as written; one signal is said in the
// singular (client D86).

import { describe, expect, test } from "bun:test";

import { kindSaid, waitingSaid } from "./inbox";

describe("the queued-signal section", () => {
  test("says each kind through the table, an unknown kind as written", () => {
    expect(["mention", "thread", "broadcast", "steer", "rumour"].map((kind) => kindSaid("en", kind))).toEqual([
      "mention",
      "thread",
      "broadcast",
      "steer",
      "rumour",
    ]);
  });

  test("says one signal in the singular and more in the plural", () => {
    expect([1, 2, 1200].map((n) => waitingSaid("en", n))).toEqual([
      "1 signal waiting for a run here",
      "2 signals waiting for a run here",
      "1,200 signals waiting for a run here",
    ]);
  });
});
