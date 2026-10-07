// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a hint's look is handed, and the one rule that builds it from the
// seat's props and state (client/Spec.lean §4-18, §7-3). No DOM and no
// runes: the seat owns the dismissal and the listeners, this file only
// says what they mean for the two elements the look draws.

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";
import type { HTMLAttributes } from "svelte/elements";

// Which side of its control a hint stands on. Above, unless the control
// stands at the window's foot with room only beside it - the edge keys -
// where it stands to the right.
export type TipSide = "above" | "right";

// How the hint is placed. A seat always leaves the choice to the engine:
// against the anchor where anchor positioning exists, against the
// wrapper where it does not. `against-wrapper` is the second branch on
// its own, so `#/gallery` can draw it in an engine that has the first
// one and the render gate measures both (client/Spec.lean §2).
export type Placing = "by-engine" | "against-wrapper";

// Whether the hint waits for hover or focus, was dismissed with Escape,
// or is drawn standing - the last only for a specimen on `#/gallery`,
// where nothing hovers and the box must be there to be measured.
export type Showing = "wanted" | "dismissed" | "standing";

// Spread on the element each one names: the wrapper hears re-engagement,
// the hint carries its id, its role and whether hold-to-reveal reaches it.
export type HolderWire = HTMLAttributes<HTMLSpanElement>;
export type HintWire = HTMLAttributes<HTMLSpanElement>;

// Everything the look is handed but the control itself, which the seat
// passes through untouched.
export interface TipWiring {
  readonly holder: HolderWire;
  readonly hint: HintWire;
  // The hint's id, handed to the control so it can write the relation.
  readonly id: string;
  // The anchor name, one per instance, so two hints on one row anchor
  // to their own controls.
  readonly anchor: string;
  readonly text: string;
  readonly side: TipSide;
  readonly placing: Placing;
  readonly showing: Showing;
}

export interface TipLook extends TipWiring {
  readonly children: Snippet<[string]>;
}

export interface TipProps {
  // Already in the person's language.
  readonly text: string;
  // The control the hint is about, handed the hint's id: as
  // `aria-describedby` when the control is named by its own text, as
  // `aria-labelledby` when these words are that name.
  readonly children: Snippet<[string]>;
  readonly side?: TipSide;
  // Whether holding the accelerator alone draws this hint with the
  // others (docs/frontend-method.md §7E): the names of the edge keys
  // are, a hint inside a form is not.
  readonly exposable?: boolean;
  // The hint's id, when the caller's wiring has already named it (a
  // part that writes `aria-describedby` into its own wire bag). Absent,
  // the seat names it.
  readonly id?: string;
}

// What the seat keeps beside its props: its own id, whether Escape has
// dismissed the hint, and the listener that brings it back.
export interface TipState {
  readonly uid: string;
  readonly dismissed: boolean;
  readonly reengage: Attachment<HTMLSpanElement>;
}

// One key for every instance: a fresh key on every look would tear the
// listeners down and put them back each time the look is rebuilt.
const REENGAGE = createAttachmentKey();

// Escape is the hint's one key (APG Tooltip, WCAG 1.4.13).
export function dismisses(key: string): boolean {
  return key === "Escape";
}

export function lookOf(props: Omit<TipProps, "children">, state: TipState): TipWiring {
  const id = props.id ?? state.uid;
  return {
    holder: { [REENGAGE]: state.reengage },
    hint: {
      id,
      role: "tooltip",
      "data-exposable": props.exposable === true && !state.dismissed ? "" : undefined,
    },
    id,
    anchor: `--tip-${id}`,
    text: props.text,
    side: props.side ?? "above",
    placing: "by-engine",
    showing: state.dismissed ? "dismissed" : "wanted",
  };
}
