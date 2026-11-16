// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The rhythm a reply arrived in, as this page saw it: how much text came
// in each slice of the time between the first and the last arrival
// (refrain §3-3).
//
// **It is an observation, not a timing.** The samples are this page's
// own clock at the moments the growing text was painted, and a frame
// that carried three deltas is one sample, so the shape says "it came in
// a steady stream" or "it stalled twice", never how many tokens arrived
// when. A reply opened from history was never watched and has no samples,
// so it has no rhythm rather than an invented one.

// One moment the growing text was seen, and how long it was then.
export interface Sample {
  readonly at: number;
  readonly length: number;
}

// How many slices the line is cut into: one per two pixels of a 40 px
// sparkline.
export const SLICES = 20;

// The share of the busiest slice each slice carried, from 0 to 1, or
// `null` when there is nothing to draw: fewer than three samples, no
// time between the first and the last, or no growth at all.
export function rhythmOf(samples: readonly Sample[]): readonly number[] | null {
  const first = samples.at(0);
  const last = samples.at(-1);
  if (samples.length < 3 || first === undefined || last === undefined) return null;
  const span = last.at - first.at;
  if (span <= 0 || last.length <= first.length) return null;
  const slices = Array.from({ length: SLICES }, () => 0);
  let before = first.length;
  for (const sample of samples.slice(1)) {
    const slice = Math.min(SLICES - 1, Math.floor(((sample.at - first.at) / span) * SLICES));
    slices[slice] = (slices[slice] ?? 0) + Math.max(0, sample.length - before);
    before = Math.max(before, sample.length);
  }
  const most = Math.max(...slices);
  return slices.map((each) => each / most);
}
