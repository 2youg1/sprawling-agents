// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { NARROWEST } from "../../core/workbench";
import type { KeyPress } from "./drawn";

export function lineAt(grid: HTMLElement, clientX: number): number {
  const frame = grid.closest<HTMLElement>(".frame") ?? grid;
  const style = getComputedStyle(frame);
  const gap = Number.parseFloat(style.columnGap) || 0;
  let edge =
    frame.getBoundingClientRect().left + (Number.parseFloat(style.paddingLeft) || 0) - gap / 2;
  const lines = style.gridTemplateColumns.split(" ").map((track) => {
    const at = edge;
    edge += Number.parseFloat(track) + gap;
    return at;
  });
  const distances = [...lines, edge].map((at) => Math.abs(clientX - at));
  return distances.indexOf(Math.min(...distances)) + 1;
}

export type Step =
  | { readonly kind: "span"; readonly span: number }
  | { readonly kind: "reset" }
  | { readonly kind: "none" };

export function stepOf(
  key: string,
  span: number,
  widest: number,
  { orientation = "vertical", minimum = NARROWEST, increment = 1 }: DividerRange = {},
): Step {
  const lesser = orientation === "vertical" ? "ArrowLeft" : "ArrowUp";
  const greater = orientation === "vertical" ? "ArrowRight" : "ArrowDown";
  switch (key) {
    case lesser:
      return { kind: "span", span: span - increment };
    case greater:
      return { kind: "span", span: span + increment };
    case "Home":
      return { kind: "span", span: minimum };
    case "End":
      return { kind: "span", span: widest };
    case "Enter":
      return { kind: "reset" };
    default:
      return { kind: "none" };
  }
}

export interface SplitterWire {
  readonly role: "separator";
  readonly tabindex: 0;
  readonly "aria-orientation": "vertical" | "horizontal";
  readonly "aria-label": string;
  readonly "aria-controls": string;
  readonly "aria-valuenow": number;
  readonly "aria-valuemin": number;
  readonly "aria-valuemax": number;
  readonly onkeydown: (event: KeyPress) => void;
  readonly ondblclick: () => void;
  readonly onpointerdown: (
    event: PointerEvent & { readonly currentTarget: EventTarget & HTMLElement },
  ) => void;
  readonly onpointermove: (event: PointerEvent) => void;
  readonly onpointerup: () => void;
  readonly onpointercancel: () => void;
}

export interface DividerLook {
  readonly wire: SplitterWire;

  readonly dragging: boolean;
}

interface DividerRange {
  readonly orientation?: "vertical" | "horizontal";
  readonly minimum?: number;
  readonly increment?: number;
}

export interface DividerState extends DividerRange {
  readonly label: string;

  readonly controls: string;
  readonly span: number;
  readonly widest: number;
  readonly dragging: boolean;
}

export interface DividerHands {
  readonly set: (span: number) => void;
  readonly reset: () => void;
  readonly drag: (dragging: boolean) => void;

  readonly follow: (point: number) => void;
}

export function dividerLookOf(state: DividerState, hands: DividerHands): DividerLook {
  const orientation = state.orientation ?? "vertical";
  return {
    dragging: state.dragging,
    wire: {
      role: "separator",
      tabindex: 0,
      "aria-orientation": orientation,
      "aria-label": state.label,
      "aria-controls": state.controls,
      "aria-valuenow": state.span,
      "aria-valuemin": state.minimum ?? NARROWEST,
      "aria-valuemax": state.widest,
      onkeydown: (event) => {
        const step = stepOf(event.key, state.span, state.widest, state);
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
        if (event.button !== 0) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        hands.drag(true);
      },
      onpointermove: (event) => {
        if (state.dragging)
          hands.follow(orientation === "vertical" ? event.clientX : event.clientY);
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
