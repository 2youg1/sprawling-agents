// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a person stood in each place they write, within this tab
// (refrain §3-14, the second row; client/Spec.lean §4-63): the selection in the
// box, and how far up the thread they were reading. The words themselves
// are the draft row's (`core/prefs.ts`). These two live in memory only,
// because they describe this window: a reload is a new reading, and a
// row per place would fill the browser's store with scroll offsets.
//
// And what the person last sent there, for ↑ in an empty box: read from
// the ledger rather than remembered here, so it survives a reload and
// covers words sent from another tab or device.

import { heldIn } from "../../core/belief/rooms";
import type { Belief } from "../../core/belief/shape";
import type { Address } from "../../wire";

interface Selection {
  readonly start: number;
  readonly end: number;
  readonly direction: "forward" | "backward" | "none";
}

const selections = new Map<string, Selection>();
// How far from the top the thread stood, for a place the person was
// reading back in; a place following its foot has no entry.
const readings = new Map<string, number>();

// A box with no place keeps nothing, and a place with no box mounted
// yet has nothing to keep.
export function keepSelection(place: string | undefined, box: HTMLTextAreaElement | undefined): void {
  if (place !== undefined && box !== undefined) {
    selections.set(place, { start: box.selectionStart, end: box.selectionEnd, direction: box.selectionDirection });
  }
}

// Puts the place's selection back into the box, or leaves the caret
// where the browser put it when this tab never stood there.
export function restoreSelection(place: string | undefined, box: HTMLTextAreaElement | undefined): void {
  const kept = place === undefined ? undefined : selections.get(place);
  if (kept !== undefined) box?.setSelectionRange(kept.start, kept.end, kept.direction);
}

export function keepReading(place: string, top: number | null): void {
  if (top === null) readings.delete(place);
  else readings.set(place, top);
}

export function readingAt(place: string): number | null {
  return readings.get(place) ?? null;
}

// The newest task the person dispatched in this room, `null` when the
// room has none. A steer's words are not in the belief, so they are not
// recalled.
export function recalled(belief: Belief, room: Address): string | null {
  return heldIn(belief, room)
    .flatMap((run) => (run.task === null || run.task === "" ? [] : [run.task]))
    .at(-1) ?? null;
}
