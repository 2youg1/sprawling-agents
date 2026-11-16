// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Turn } from "../../wire";
import { tpsOf, ttftOf } from "../talk/timing";

export interface Speed {
  readonly ttft: number;
  readonly ttftMean: number;
  readonly turns: number;
  readonly tps: Spread | null;
}

export interface Spread {
  readonly p50: number;
  readonly p1: number;
  readonly turns: number;
}

export function speedOf(turns: readonly Turn[]): Speed | null {
  const ttfts = measuredBy(turns, ttftOf);
  const ttft = rankOf(ttfts, 50);
  const mean = meanOf(ttfts);
  if (ttft === null || mean === null) return null;
  const rates = measuredBy(turns, tpsOf);
  const p50 = rankOf(rates, 50);
  const p1 = rankOf(rates, 1);
  return {
    ttft: Math.round(ttft),
    ttftMean: Math.round(mean),
    turns: ttfts.length,
    tps: p50 === null || p1 === null ? null : { p50, p1, turns: rates.length },
  };
}

function measuredBy(
  turns: readonly Turn[],
  figure: (turn: Turn) => number | null,
): readonly number[] {
  return turns.flatMap((turn) => {
    const value = figure(turn);
    return value === null ? [] : [value];
  });
}

export function rankOf(values: readonly number[], percent: number): number | null {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.max(Math.ceil((percent / 100) * sorted.length), 1) - 1] ?? null;
}

export function meanOf(values: readonly number[]): number | null {
  return values.length === 0 ? null : values.reduce((sum, value) => sum + value, 0) / values.length;
}
