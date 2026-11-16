// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The board's clock is near-sighted on purpose: the last five minutes
// take the right half of a run's bar, the ten before them a quarter,
// and the rest of the hour the first quarter. A board is read for what
// is happening now, and on a straight hour a thirty-second tool call is
// a pixel wide. The scale header draws the folds so the change of
// scale is seen rather than hidden.

export interface Fold {
  // Minutes before now, and the share of the bar (0 to 100) at which
  // that moment is drawn.
  readonly minutes: number;
  readonly at: number;
}

export const FOLDS: readonly Fold[] = [
  { minutes: 60, at: 0 },
  { minutes: 15, at: 25 },
  { minutes: 5, at: 50 },
  { minutes: 0, at: 100 },
];

// A moment as a position along the folded bar, in per cent, clamped to
// its two edges.
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
