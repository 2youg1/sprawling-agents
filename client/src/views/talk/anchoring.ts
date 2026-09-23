// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the conversation view is anchored while words arrive (ux B1).
//
// A reply streams in at the foot of the thread. While the person is at
// the foot, the view follows the growing edge; the moment they scroll
// up to read, their place is theirs and the view holds it. CSS
// `overflow-anchor` cannot decide this: the growth appends at the end,
// so what has to stay in view is a place in text that is already
// there, not an element that moved.
//
// The judgement answers "was the viewport at the foot before this
// append", so the caller records it on every scroll and reads it again
// before following. The distance from the foot is
// `scrollHeight - scrollTop - clientHeight`, and a distance below
// `FOOT_TOLERANCE` counts as the foot. Exactly at the tolerance is
// already a place the person chose, so it holds.
//
// This is the whole rule; `talk.svelte` owns the scroller and is the
// only reader of the DOM.

// The three measurements of a scroller, which travel together: one
// without the other two decides nothing.
export interface Foot {
  readonly scrollTop: number;
  readonly clientHeight: number;
  readonly scrollHeight: number;
}

// What the view does with the growing edge. The two spellings are the
// whole answer, so a caller cannot hold a third behaviour.
export type Anchoring = "follow" | "hold";

// How close to the foot counts as being at the foot, in pixels. Past
// this the person has scrolled up to read, and their place is theirs.
export const FOOT_TOLERANCE = 40;

// Read the three measurements a judgement is made from. The scrolling
// box itself is an Element, and this is the only DOM read here.
export function footOf(box: Element): Foot {
  return {
    scrollTop: box.scrollTop,
    clientHeight: box.clientHeight,
    scrollHeight: box.scrollHeight,
  };
}

// Decide whether the growing edge keeps the viewport where it is. A
// viewport past the foot - rubber-band scrolling on its way back -
// reads as the foot rather than as a failure.
export function anchorAt(foot: Foot): Anchoring {
  return foot.scrollHeight - foot.scrollTop - foot.clientHeight < FOOT_TOLERANCE
    ? "follow"
    : "hold";
}
