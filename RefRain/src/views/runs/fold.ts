// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface Fold {
  readonly minutes: number;
  readonly at: number;
}

export const FOLDS: readonly Fold[] = [
  { minutes: 60, at: 0 },
  { minutes: 15, at: 25 },
  { minutes: 5, at: 50 },
  { minutes: 0, at: 100 },
];

export function along(at: number, now: number): number {
  const before = (now - at) / 60_000;
  for (let n = 1; n < FOLDS.length; n += 1) {
    const far = FOLDS[n - 1];
    const near = FOLDS[n];
    if (far !== undefined && near !== undefined && before >= near.minutes) {
      const share = Math.min(1, (far.minutes - before) / (far.minutes - near.minutes));
      return Math.max(0, far.at + share * (near.at - far.at));
    }
  }
  return 100;
}
