// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Hunk, Sign } from "./trace";

export interface Reverse {
  readonly now: string;
  readonly was: string;
  readonly fence: string;
}

export function reverseOf(hunk: Hunk): Reverse {
  const now = sideOf(hunk, "added");
  const was = sideOf(hunk, "removed");
  const longest = Math.max(
    0,
    ...[now, was].flatMap((side) => (side.match(/`+/g) ?? []).map((run) => run.length)),
  );
  return { now, was, fence: "`".repeat(Math.max(3, longest + 1)) };
}

export function lineOf(hunk: Hunk): number {
  const first = hunk.lines.find((line) => line.new !== null) ?? hunk.lines.at(0);
  return first?.new ?? first?.old ?? 1;
}

function sideOf(hunk: Hunk, side: Exclude<Sign, "kept">): string {
  return hunk.lines
    .filter((line) => line.sign === "kept" || line.sign === side)
    .map((line) => line.text)
    .join("\n");
}
