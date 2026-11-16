// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface Sample {
  readonly at: number;
  readonly length: number;
}

export const SLICES = 20;

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
