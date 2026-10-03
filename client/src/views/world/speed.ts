// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the two speed cells of the session's sheet say (client/Spec.lean
// §7K): over the turns that measured a time to first content, its median
// and its mean and how many turns that is; over the turns that measured
// an output rate (`tpsOf`), its 50th and 99th percentile and how many
// turns that is, absent when none did.
//
// The median is the figure a person compares sessions by, because one
// turn that waited on a cold provider would otherwise move it; the mean
// stands beside it because that cold turn is also a fact the person
// pays for. The rate's 99th percentile is the fastest stretch the
// provider gave, and the gap to the 50th says how even it was. Every
// percentile is a nearest rank - a value one turn actually measured,
// never an interpolation between two. A turn whose times are not
// measurements (`Turn.timing`), or that streamed nothing, has no figure
// and is left out rather than counted as zero.

import type { Turn } from "../../wire";
import { tpsOf, ttftOf } from "../talk/timing";

export interface Speed {
  // Milliseconds, rounded.
  readonly ttft: number;
  readonly ttftMean: number;
  readonly turns: number;
  readonly tps: Spread | null;
}

// Output tokens a second at two ranks, over `turns` measured turns.
export interface Spread {
  readonly p50: number;
  readonly p99: number;
  readonly turns: number;
}

export function speedOf(turns: readonly Turn[]): Speed | null {
  const ttfts = measuredBy(turns, ttftOf);
  const ttft = rankOf(ttfts, 50);
  const mean = meanOf(ttfts);
  if (ttft === null || mean === null) return null;
  const rates = measuredBy(turns, tpsOf);
  const p50 = rankOf(rates, 50);
  const p99 = rankOf(rates, 99);
  return {
    ttft: Math.round(ttft),
    ttftMean: Math.round(mean),
    turns: ttfts.length,
    tps: p50 === null || p99 === null ? null : { p50, p99, turns: rates.length },
  };
}

function measuredBy(turns: readonly Turn[], figure: (turn: Turn) => number | null): readonly number[] {
  return turns.flatMap((turn) => {
    const value = figure(turn);
    return value === null ? [] : [value];
  });
}

// The nearest-rank percentile: the smallest measured value with at
// least `percent` per cent of the values at or below it.
export function rankOf(values: readonly number[], percent: number): number | null {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.max(Math.ceil((percent / 100) * sorted.length), 1) - 1] ?? null;
}

export function meanOf(values: readonly number[]): number | null {
  return values.length === 0 ? null : values.reduce((sum, value) => sum + value, 0) / values.length;
}
