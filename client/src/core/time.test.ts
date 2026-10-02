// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { isoDay, isoInstant, isoTime, kilo } from "./time";

describe("a token count read at a glance", () => {
  test("three significant figures under a mark, and no trailing zero", () => {
    expect([999, 1_000, 82_400, 200_000, 1_240_000, 12_345, 999_999].map(kilo)).toEqual([
      "999",
      "1k",
      "82.4k",
      "200k",
      "1.24M",
      "12.3k",
      "1M",
    ]);
  });
});

describe("a ledger instant in UTC", () => {
  test("the day once, then the time of day to the millisecond", () => {
    const at = Date.UTC(2026, 9, 2, 3, 4, 5, 678);
    expect([isoDay(at), isoTime(at)]).toEqual(["2026-10-02", "03:04:05.678Z"]);
  });
  test("the whole instant a datetime attribute carries is the two halves joined", () => {
    const at = Date.UTC(2026, 9, 2, 3, 4, 5, 678);
    expect(isoInstant(at)).toBe("2026-10-02T03:04:05.678Z");
  });
});
