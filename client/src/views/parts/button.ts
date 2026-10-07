// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the one button decides before anything is drawn: which posture
// it is in, and everything the element it is drawn as must carry - its
// type, its two ARIA properties, the relation to the hint that says why
// it cannot be used, and the guard that keeps a press from landing while
// it may not. The look (`button.look.svelte`, or any other look that
// takes `ButtonLook`) spreads that bag on its `<button>` and writes no
// ARIA and no handler of its own, so a look from a component library
// keeps the same keyboard and screen-reader contract
// (client/spec/Views/Parts.lean §7-2).
//
// Four tones, because a page has four kinds of thing to ask for; a
// loading posture, because a command frame is answered later than the
// hand that sent it; and a refusal that carries its reason, because a
// control a person cannot use owes them the sentence saying why. The
// words are the caller's: this file holds no prose.

// primary is the one thing the screen is for, secondary the ones beside
// it, quiet the ones that must not compete, destructive the one that
// cannot be taken back.
export type Tone = "primary" | "secondary" | "quiet" | "destructive";

// What the control is doing. Every posture the button can take is a
// value here, so a look reads one word rather than deducing the state
// from which properties happen to be set.
export type ButtonState = "idle" | "loading" | "stopped";

export interface ButtonProps {
  // Already in the person's language: a `say` key resolved by the page.
  readonly label: string;
  readonly tone?: Tone;
  readonly type?: "button" | "submit";
  // The command went out and its acknowledgement has not come back.
  readonly loading?: boolean;
  // Present means the control cannot be used, and says why. It stays
  // focusable and keeps `aria-disabled`, because a `disabled` element is
  // skipped by the keyboard and its reason is never read out. The reason
  // is drawn beside the button rather than left in a `title`, which a
  // keyboard never reaches and a touch screen never shows.
  readonly why?: string;
  readonly onPress?: () => void;
}

// Spread on the `<button>` the look draws. Enter and Space are both
// delivered as a click by the platform, so the guard inside `onclick` is
// what stands between every input and `onPress`.
export interface ButtonWire {
  readonly type: "button" | "submit";
  readonly "aria-disabled": boolean;
  readonly "aria-busy": boolean;
  readonly "aria-describedby": string | undefined;
  readonly onclick: () => void;
}

export interface ButtonLook {
  readonly label: string;
  readonly tone: Tone;
  readonly state: ButtonState;
  // Present means the look draws the reason in a `Tip` around the button.
  readonly why: string | undefined;
  // The bag for the `<button>`, given the id of the `Tip` that carries
  // `why` (the `Tip` names its own hint), or `undefined` when there is no
  // reason to point at.
  readonly wire: (hint: string | undefined) => ButtonWire;
}

// The one reading of what the control is doing. The look, the two ARIA
// properties and the click guard all read this, and nothing else
// decides the question.
export function stateOf(props: ButtonProps): ButtonState {
  if (props.why !== undefined) return "stopped";
  return props.loading === true ? "loading" : "idle";
}

export function lookOf(props: ButtonProps): ButtonLook {
  const state = stateOf(props);
  return {
    label: props.label,
    tone: props.tone ?? "secondary",
    state,
    why: props.why,
    wire: (hint) => ({
      type: props.type ?? "button",
      "aria-disabled": state !== "idle",
      "aria-busy": state === "loading",
      "aria-describedby": hint,
      onclick: () => {
        if (state !== "idle") return;
        props.onPress?.();
      },
    }),
  };
}
