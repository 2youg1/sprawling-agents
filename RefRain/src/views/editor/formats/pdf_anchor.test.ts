// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { cropPixels, readableRegion, regionBetween } from "./pdf_anchor";

describe("PDF page coordinates", () => {
  test("reverse drag selects the same area on the page", () => {
    const topLeft = { x: 0.25, y: 0.125 };
    const bottomRight = { x: 0.75, y: 0.5 };
    expect(regionBetween(bottomRight, topLeft)).toEqual({
      x: 0.25,
      y: 0.125,
      width: 0.5,
      height: 0.375,
    });
  });

  test("a pixel touched by the edge stays out of the crop", () => {
    expect(cropPixels({ x: 0.23, y: 0.23, width: 0.34, height: 0.34 }, 10, 10)).toEqual({
      x: 3,
      y: 3,
      width: 2,
      height: 2,
    });
    expect(cropPixels({ x: 0.12, y: 0.13, width: 0.01, height: 0.01 }, 10, 10)).toBeNull();
  });

  test("invalid or out-of-page coordinates never resolve", () => {
    expect(readableRegion({ x: 0.8, y: 0, width: 0.3, height: 0.6 })).toBe(false);
    expect(readableRegion({ x: 0, y: Number.NaN, width: 1, height: 1 })).toBe(false);
    expect(cropPixels({ x: 0, y: 0, width: 1, height: 1 }, 10, 10)).toEqual({
      x: 0,
      y: 0,
      width: 10,
      height: 10,
    });
  });
});
