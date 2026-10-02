// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { meanOf, rankOf } from "./speed";

describe("the speed cell's ranks", () => {
  test("a percentile is a value some turn measured, by nearest rank", () => {
    const rates = [40, 10, 30, 20];
    expect([rankOf(rates, 50), rankOf(rates, 99), rankOf([7], 99), rankOf([], 50)]).toEqual([20, 40, 7, null]);
  });

  test("one cold turn moves the mean and leaves the median", () => {
    const ttfts = [300, 320, 310, 4000];
    expect([rankOf(ttfts, 50), meanOf(ttfts), meanOf([])]).toEqual([310, 1232.5, null]);
  });
});
