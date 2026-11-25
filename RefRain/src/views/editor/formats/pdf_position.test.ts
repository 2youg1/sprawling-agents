// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { PdfPageHeights } from "./pdf_position";

test("the first measured page estimates unknown heights without moving an anchor to another page", () => {
  const heights = new PdfPageHeights(2_000);
  const width = 360;
  const gap = 16;
  const page = 1_900;
  heights.measure(page, { width: 400, height: 600 });
  expect(heights.top(page, width, gap)).toBe((page - 1) * (540 + gap));

  heights.measure(1, { width: 600, height: 400 });
  heights.measure(100, { width: 400, height: 800 });
  const top = heights.top(page, width, gap);
  expect(top).toBe((page - 1) * (540 + gap) + (240 - 540) + (720 - 540));
  expect(heights.position(top + 135, width, gap)).toEqual({ page, offset: 0.25 });
  expect(heights.top(2_001, width, gap) - heights.top(2_000, width, gap)).toBe(540 + gap);
});

test("indexed PDF positions agree with a direct layout across mixed page heights", () => {
  const count = 1_025;
  const heights = new PdfPageHeights(count);
  const width = 771.55;
  const gap = 19.5;
  const natural = { width: 612, height: 792 };
  const measured = new Map([[1, natural]]);
  heights.measure(1, natural);

  let seed = 137;
  const sample = () => {
    seed = (seed * 48_271) % 2_147_483_647;
    return seed;
  };
  for (let step = 0; step < count; step++) {
    const page = (sample() % count) + 1;
    const size = { width: (sample() % 1200) + 5, height: (sample() % 1800) + 5 };
    measured.set(page, size);
    heights.measure(page, size);
  }

  let top = 0;
  for (let page = 1; page <= count; page++) {
    const size = measured.get(page) ?? natural;
    const height = (size.height / size.width) * width;
    expect(heights.top(page, width, gap)).toBeCloseTo(top, 5);
    const point = heights.position(top + height * 0.37, width, gap);
    expect(point.page).toBe(page);
    expect(point.offset).toBeCloseTo(0.37, 8);
    expect(heights.pageAt(top + height + gap / 2, width, gap)).toBe(page);
    top += height + gap;
  }
  expect(heights.top(count + 1, width, gap)).toBeCloseTo(top, 5);
});
