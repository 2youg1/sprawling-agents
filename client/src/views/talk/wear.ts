// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the read-wear bar draws (`wear.svelte` asks the page, and
// `wear.look.svelte` draws): the stretches of the conversation the
// viewport has shown, kept as a short sorted list of disjoint pieces
// as the view moves, and the value the look is given.

import type { Phase } from "../runs/lineage";
import { PHASES } from "../runs/phase";

// A stretch of the content, as its start and its end, or - once given
// to the look - as its top and its height in shares of the whole.
export type Stretch = readonly [number, number];

// One round's tick on the track, in shares of the whole content.
export interface Mark {
  readonly top: number;
  readonly height: number;
  readonly phase: Phase;
}

// The handlers the track carries, spread onto it as they are: pressing
// moves the view there and dragging follows the pointer.
export interface TrackWire {
  readonly onpointerdown: (event: PointerEvent & { currentTarget: EventTarget & HTMLElement }) => void;
  readonly onpointermove: (event: PointerEvent & { currentTarget: EventTarget & HTMLElement }) => void;
}

// Everything the bar's look is given: the words a resting pointer is
// told, the track's handlers, and the three layers in shares of the
// whole content - what has been read, where the view is, and where
// each round is.
export interface WearLook {
  readonly hint: string;
  readonly track: TrackWire;
  readonly read: readonly Stretch[];
  readonly thumb: Stretch;
  readonly marks: readonly Mark[];
}

// The read stretches with `from`..`to` worn in: every stretch it
// touches or overlaps is merged into one, and the list stays sorted
// and disjoint, so it stays as short as the reading was scattered.
export function worn(seen: readonly Stretch[], from: number, to: number): readonly Stretch[] {
  const apart = seen.filter(([a, b]) => b < from || a > to);
  const joined = seen.filter(([a, b]) => !(b < from || a > to));
  const start = Math.min(from, ...joined.map(([a]) => a));
  const end = Math.max(to, ...joined.map(([, b]) => b));
  return [...apart, [start, end] as const].sort((x, y) => x[0] - y[0]);
}

// The phase a block's `data-wear` attribute names, or undefined for a
// word that is not one, which the bar then leaves undrawn.
export function phaseNamed(word: string | undefined): Phase | undefined {
  return PHASES.find((each) => each === word);
}
