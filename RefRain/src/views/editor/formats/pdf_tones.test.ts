// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { neutralTones } from "./pdf_tones";

test("the tone overlay keeps saturated formula and chart pixels transparent", () => {
  const data = new Uint8ClampedArray([
    0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255, 120, 120, 135, 180,
  ]);
  neutralTones({ data, width: 5, height: 1 }, false);
  expect([...data.subarray(0, 8)]).toEqual([0, 0, 0, 255, 255, 255, 255, 255]);
  expect([data[11], data[15], data[19]]).toEqual([0, 0, 165]);
});

test("the scan pass sharpens neutral luminance within neighbouring bounds", () => {
  const data = new Uint8ClampedArray(
    [250, 20, 250, 120, 120, 200, 250, 200, 250].flatMap((value) => [value, value, value, 255]),
  );
  const baseline = data.slice();
  neutralTones({ data, width: 3, height: 3 }, true);
  neutralTones({ data: baseline, width: 3, height: 3 }, false);
  expect(data[16] ?? 0).toBeLessThan(baseline[16] ?? 0);
  expect(data[16] ?? 0).toBeGreaterThan(baseline[4] ?? 0);
  expect(data[16]).toBe(data[17]);
  expect(data[16]).toBe(data[18]);
  expect(data[19]).toBe(255);
});
