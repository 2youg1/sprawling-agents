// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

type Hold = (element: HTMLElement) => void | (() => void);

export interface SplitProps {
  readonly lines: number;
  readonly least: number;
  readonly most: number;

  readonly controls: string;

  readonly step: number;
  readonly onLines: (lines: number) => void;
  readonly onReset: () => void;
}

export interface Grip {
  readonly y: number;
  readonly lines: number;
}

export interface SplitHands {
  readonly capture: (pointer: number) => void;
  readonly hold: Hold;
  readonly grip: { from: Grip | null };
}

export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;
export type PointerPress = Pick<PointerEvent, "clientY" | "pointerId">;

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
}

export interface SplitLook {
  readonly hold: Hold;
  readonly wire: SplitWire;
}

export type Keyed =
  { readonly kind: "lines"; readonly lines: number } | { readonly kind: "reset" } | undefined;

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

export function dragged(
  props: Pick<SplitProps, "least" | "most" | "step">,
  from: Grip,
  y: number,
): number {
  return clamped(props, from.lines + Math.round((y - from.y) / props.step));
}

export function lookOf(props: SplitProps, label: string, hands: SplitHands): SplitLook {
  const { grip } = hands;
  return {
    hold: hands.hold,
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
    },
  };
}
