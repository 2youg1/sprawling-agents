// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a person's action on one hunk needs to know about it.
//
// A revert goes through the edit tool the agent already has, so it is
// fenced by the file's current version and lands in the ledger like any
// other edit; the page itself writes no file. The tool replaces `old`
// with `new`, which for a revert is the hunk read backwards: the file as
// it is now becomes the file as it was.

import type { Hunk, Sign } from "./trace";

export interface Reverse {
  // The hunk's kept and added lines: what the file holds now.
  readonly now: string;
  // The hunk's kept and removed lines: what the file held before.
  readonly was: string;
}

export function reverseOf(hunk: Hunk): Reverse {
  return { now: sideOf(hunk, "added"), was: sideOf(hunk, "removed") };
}

// The hunk's first line in the file as it is now. A hunk that only
// removed lines has no line there, and opens where its lines stood.
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
