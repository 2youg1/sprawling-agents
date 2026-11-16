// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What stands where a list has nothing in it, decided before it is
// drawn: a shape the eye lands on, one sentence saying what is missing,
// and the one action that ends the emptiness. A grey word on its own
// leaves a person unsure whether the page is empty or broken, so the
// shape is always there - the page's own outline when it has one worth
// drawing, the empty ring when it does not - and the action is the one
// part that may be absent (client/Spec.lean 4-10).
//
// The shape is decoration and is hidden from a screen reader; the
// sentence and the action are the whole readable content. The sentence
// arrives as a key rather than as a word, so what a person is told here
// has exactly one home, `lang.json`, like every other word on a screen.

import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";

export interface EmptyStateProps {
  // The key of the sentence saying what this screen is missing.
  readonly missing: Key;
  // The outline the eye lands on, when the page has one worth drawing.
  readonly shape?: Snippet;
  // Usually one Button: the way out of the emptiness.
  readonly action?: Snippet;
  // Where the sentence sits. `centred` stands alone in a narrow column;
  // `region` stands in for a list or a table on a wide page, so it is
  // drawn where that region would be - its left edge, its width, and a
  // dashed outline of it - rather than as a line floating in the middle
  // of the window. `inset` is the same seat inside a container that
  // already draws the region's outline, such as an empty table's frame.
  readonly seat?: EmptySeat;
}

export type EmptySeat = "centred" | "region" | "inset";

// Spread on the box the shape is drawn in.
export interface ShapeWire {
  readonly "aria-hidden": "true";
}

export interface EmptyLook {
  readonly seat: EmptySeat;
  readonly shapeWire: ShapeWire;
  // Undefined draws the look's own mark.
  readonly shape: Snippet | undefined;
  // Already in the person's language.
  readonly sentence: string;
  readonly action: Snippet | undefined;
}

export function lookOf(props: EmptyStateProps, sentence: string): EmptyLook {
  return {
    seat: props.seat ?? "centred",
    shapeWire: { "aria-hidden": "true" },
    shape: props.shape,
    sentence,
    action: props.action,
  };
}
