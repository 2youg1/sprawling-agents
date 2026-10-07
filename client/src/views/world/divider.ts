// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a workbench divider's look is given (`divider.look.svelte`), and
// the wiring the seat (`divider.svelte`) builds for it: APG Window
// Splitter (client/spec/Views/Workspace.lean §7-11). Left and Right move
// the edge one column, Home and End to the narrowest and widest the pane
// before it may be, Enter and a double click put the workbench back the
// way it ships; the clamping is `core/workbench.ts`'s `resized`, whose
// model is client/spec/Core/Workbench.lean.

import { NARROWEST } from "../../core/workbench";
import type { KeyPress } from "./drawn";

// What a key on the divider asks for: the pane before it at `span`
// columns, the workbench as it ships, or nothing.
export type Step = { readonly kind: "span"; readonly span: number } | { readonly kind: "reset" } | { readonly kind: "none" };

export function stepOf(key: string, span: number, widest: number): Step {
  switch (key) {
    case "ArrowLeft":
      return { kind: "span", span: span - 1 };
    case "ArrowRight":
      return { kind: "span", span: span + 1 };
    case "Home":
      return { kind: "span", span: NARROWEST };
    case "End":
      return { kind: "span", span: widest };
    case "Enter":
      return { kind: "reset" };
    default:
      return { kind: "none" };
  }
}

// The bag spread on the separator. The pointer handlers capture the
// pointer on the separator, so a drag that leaves it keeps moving it.
export interface SplitterWire {
  readonly role: "separator";
  readonly tabindex: 0;
  readonly "aria-orientation": "vertical";
  readonly "aria-label": string;
  readonly "aria-controls": string;
  readonly "aria-valuenow": number;
  readonly "aria-valuemin": number;
  readonly "aria-valuemax": number;
  readonly onkeydown: (event: KeyPress) => void;
  readonly ondblclick: () => void;
  readonly onpointerdown: (event: PointerEvent & { readonly currentTarget: EventTarget & HTMLElement }) => void;
  readonly onpointermove: (event: PointerEvent) => void;
  readonly onpointerup: () => void;
  readonly onpointercancel: () => void;
}

export interface DividerLook {
  readonly wire: SplitterWire;
  // Whether a pointer is dragging it now, which lights it as hover and
  // focus do.
  readonly dragging: boolean;
}

export interface DividerState {
  // Already in the person's language: "width of <pane>".
  readonly label: string;
  // The id of the pane before the divider.
  readonly controls: string;
  readonly span: number;
  readonly widest: number;
  readonly dragging: boolean;
}

export interface DividerHands {
  // The pane before the divider at `span` columns; the seat clamps.
  readonly set: (span: number) => void;
  readonly reset: () => void;
  readonly drag: (dragging: boolean) => void;
  // A pointer moved to `clientX` while dragging.
  readonly follow: (clientX: number) => void;
}

export function dividerLookOf(state: DividerState, hands: DividerHands): DividerLook {
  return {
    dragging: state.dragging,
    wire: {
      role: "separator",
      tabindex: 0,
      "aria-orientation": "vertical",
      "aria-label": state.label,
      "aria-controls": state.controls,
      "aria-valuenow": state.span,
      "aria-valuemin": NARROWEST,
      "aria-valuemax": state.widest,
      onkeydown: (event) => {
        const step = stepOf(event.key, state.span, state.widest);
        switch (step.kind) {
          case "span":
            hands.set(step.span);
            break;
          case "reset":
            hands.reset();
            break;
          case "none":
            return;
        }
        event.preventDefault();
      },
      ondblclick: hands.reset,
      onpointerdown: (event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        hands.drag(true);
      },
      onpointermove: (event) => {
        if (state.dragging) hands.follow(event.clientX);
      },
      onpointerup: () => {
        hands.drag(false);
      },
      onpointercancel: () => {
        hands.drag(false);
      },
    },
  };
}
