// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The read stretches the wear bar keeps stay a sorted list of disjoint
// pieces whose union is exactly what the viewport has shown, however
// the view wandered.

import { expect, test } from "bun:test";
import { worn } from "./wear";
import type { Stretch } from "./wear";

test("a view that wanders keeps the read stretches sorted, disjoint and whole", () => {
  // A fixed walk of views, as a person reads, jumps and comes back.
  const views: readonly Stretch[] = [
    [0, 100], [80, 180], [900, 1000], [400, 500], [480, 920], [2000, 2100], [150, 400], [1990, 2005],
  ];
  const seen = views.reduce<readonly Stretch[]>((kept, [from, to]) => worn(kept, from, to), []);
  expect(seen).toEqual([[0, 1000], [1990, 2100]]);
  for (const [at, [, end]] of seen.slice(0, -1).entries()) {
    expect(end).toBeLessThan(seen[at + 1]?.[0] ?? Infinity);
  }
});

test("a view that only touches a stretch joins it rather than leaving a seam", () => {
  expect(worn([[0, 100], [300, 400]], 100, 300)).toEqual([[0, 400]]);
});
