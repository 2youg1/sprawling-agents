// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the line between the editor and the terminal decides, apart from
// how it is drawn (client D95, the APG Window Splitter row of
// client/Spec.lean §7-11): where a key or a drag moves it, counted in
// whole lines of the code below it, and the wire bag that carries its
// role, values and handlers onto the element a look draws. Nothing here
// touches the DOM.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

export interface SplitProps {
  // How many lines the editor region holds now, and the most and least
  // it may hold.
  readonly lines: number;
  readonly least: number;
  readonly most: number;
  // The id of the editor region this line sizes.
  readonly controls: string;
  // The pixel height of one line, which a drag is counted in.
  readonly step: number;
  readonly onLines: (lines: number) => void;
  readonly onReset: () => void;
}

// Where a drag started: the pointer's height and the lines held then.
export interface Grip {
  readonly y: number;
  readonly lines: number;
}

// What only the seat can do: capture the pointer on the drawn line, so
// a drag that leaves it keeps moving it. The drag in progress lives in
// the seat as well, because the look is drawn again on every line the
// drag moves and must not forget where the drag began.
export interface SplitHands {
  readonly capture: (pointer: number) => void;
  readonly hold: Attachment<HTMLElement>;
  readonly grip: { from: Grip | null };
}

// What the handlers read from an event, so the wiring test can press a
// key or move a pointer without a DOM.
export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;
export type PointerPress = Pick<PointerEvent, "clientY" | "pointerId">;

// The bag spread on the line itself.
export interface SplitWire {
  readonly role: "separator";
  readonly tabindex: 0;
  readonly "aria-orientation": "horizontal";
  readonly "aria-label": string;
  readonly "aria-controls": string;
  readonly "aria-valuenow": number;
  readonly "aria-valuemin": number;
  readonly "aria-valuemax": number;
  readonly onkeydown: (event: KeyPress) => void;
  readonly onpointerdown: (event: PointerPress) => void;
  readonly onpointermove: (event: PointerPress) => void;
  readonly onpointerup: () => void;
  readonly onpointercancel: () => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// Everything a look is given.
export interface SplitLook {
  readonly wire: SplitWire;
}

// A key's move: to a number of lines, back to the regions' own
// proportions, or nothing for a key the line does not take.
export type Keyed = { readonly kind: "lines"; readonly lines: number } | { readonly kind: "reset" } | undefined;

function clamped(props: Pick<SplitProps, "least" | "most">, lines: number): number {
  return Math.min(props.most, Math.max(props.least, lines));
}

export function keyed(props: Pick<SplitProps, "lines" | "least" | "most">, key: string): Keyed {
  switch (key) {
    case "ArrowUp":
      return { kind: "lines", lines: clamped(props, props.lines - 1) };
    case "ArrowDown":
      return { kind: "lines", lines: clamped(props, props.lines + 1) };
    case "Home":
      return { kind: "lines", lines: props.least };
    case "End":
      return { kind: "lines", lines: props.most };
    case "Enter":
      return { kind: "reset" };
    default:
      return undefined;
  }
}

// The lines a drag has reached: the lines held when it began, moved by
// whole steps of the pointer's travel.
export function dragged(props: Pick<SplitProps, "least" | "most" | "step">, from: Grip, y: number): number {
  return clamped(props, from.lines + Math.round((y - from.y) / props.step));
}

const HOLD = createAttachmentKey();

// The whole value a look draws.
export function lookOf(props: SplitProps, label: string, hands: SplitHands): SplitLook {
  const { grip } = hands;
  return {
    wire: {
      role: "separator",
      tabindex: 0,
      "aria-orientation": "horizontal",
      "aria-label": label,
      "aria-controls": props.controls,
      "aria-valuenow": props.lines,
      "aria-valuemin": props.least,
      "aria-valuemax": props.most,
      onkeydown: (event) => {
        const moved = keyed(props, event.key);
        if (moved === undefined) return;
        event.preventDefault();
        if (moved.kind === "reset") props.onReset();
        else props.onLines(moved.lines);
      },
      onpointerdown: (event) => {
        hands.capture(event.pointerId);
        grip.from = { y: event.clientY, lines: props.lines };
      },
      onpointermove: (event) => {
        if (grip.from === null) return;
        props.onLines(dragged(props, grip.from, event.clientY));
      },
      onpointerup: () => {
        grip.from = null;
      },
      onpointercancel: () => {
        grip.from = null;
      },
      [HOLD]: hands.hold,
    },
  };
}
