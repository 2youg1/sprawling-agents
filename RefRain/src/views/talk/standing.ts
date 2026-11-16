// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { heldIn } from "../../core/belief/rooms";
import type { Belief } from "../../core/belief/shape";
import type { Address } from "../../wire";

interface Selection {
  readonly start: number;
  readonly end: number;
  readonly direction: "forward" | "backward" | "none";
}

const selections = new Map<string, Selection>();

const readings = new Map<string, number>();

export function keepSelection(
  place: string | undefined,
  box: HTMLTextAreaElement | undefined,
): void {
  if (place !== undefined && box !== undefined) {
    selections.set(place, {
      start: box.selectionStart,
      end: box.selectionEnd,
      direction: box.selectionDirection,
    });
  }
}

export function restoreSelection(
  place: string | undefined,
  box: HTMLTextAreaElement | undefined,
): void {
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

export function recalled(belief: Belief, room: Address): string | null {
  return (
    heldIn(belief, room)
      .flatMap((run) => (run.task === null || run.task === "" ? [] : [run.task]))
      .at(-1) ?? null
  );
}
