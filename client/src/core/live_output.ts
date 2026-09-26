// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a running command has written so far, as this page holds it
// (runtime-SPEC 8-28-3). A preview: the call's result in the Ledger is
// the authority on that output, so a run's tail is dropped the moment
// its `tool_result` arrives. Bounded by lines, because the page draws
// lines, and the oldest are dropped first and counted, so the terminal
// can say how many it no longer shows.

import type { LiveOutput } from "../wire";

export const LIVE_LINES = 400;

export interface Tail {
  readonly out: string;
  readonly err: string;
  readonly cut: number;
}

export const NO_TAIL: Tail = { out: "", err: "", cut: 0 };

// The tail with one more piece on the end of its stream.
export function appended(tail: Tail, piece: LiveOutput, cap: number = LIVE_LINES): Tail {
  const [text, cut] = bounded((piece.stream === "out" ? tail.out : tail.err) + piece.text, cap);
  return piece.stream === "out"
    ? { ...tail, out: text, cut: tail.cut + cut }
    : { ...tail, err: text, cut: tail.cut + cut };
}

function bounded(text: string, _cap: number): [string, number] {
  return [text, 0];
}
