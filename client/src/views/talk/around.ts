// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which turns a letter draws of the run that sent it (client D91): the
// turn that holds the line, found as the last turn opened at or before
// it, and `AROUND` turns on either side. A line the thread cannot place -
// none given, or earlier than the first turn - draws every turn, as the
// thread always did.

import type { Seq, Turn } from "../../wire";

export const AROUND = 1;

// The turns drawn are `turns.slice(from, to)`; `cut` says whether any
// were left out, so the thread can point at the whole conversation.
export interface Window {
  readonly from: number;
  readonly to: number;
  readonly cut: boolean;
}

export function turnsAround(turns: readonly Turn[], line: Seq | null): Window {
  const whole: Window = { from: 0, to: turns.length, cut: false };
  if (line === null) return whole;
  const holding = turns.reduce((found, turn, at) => (turn.opened <= line ? at : found), -1);
  if (holding < 0) return whole;
  const from = Math.max(0, holding - AROUND);
  const to = Math.min(turns.length, holding + AROUND + 1);
  return { from, to, cut: from > 0 || to < turns.length };
}
