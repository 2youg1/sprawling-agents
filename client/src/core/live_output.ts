// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { LiveOutput } from "../wire";

export const LIVE_LINES = 400;

export interface Tail {
  readonly out: string;
  readonly err: string;
  readonly cut: number;
}

export const NO_TAIL: Tail = { out: "", err: "", cut: 0 };

export function appended(tail: Tail, piece: LiveOutput, cap: number = LIVE_LINES): Tail {
  const [text, cut] = bounded((piece.stream === "out" ? tail.out : tail.err) + piece.text, cap);
  return piece.stream === "out"
    ? { ...tail, out: text, cut: tail.cut + cut }
    : { ...tail, err: text, cut: tail.cut + cut };
}

function bounded(text: string, cap: number): [string, number] {
  const lines = text.split("\n");
  const cut = Math.max(0, lines.length - cap);
  return cut === 0 ? [text, 0] : [lines.slice(cut).join("\n"), cut];
}
