// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one button in the client. Four tones, because a page has four
// kinds of thing to ask for; a loading posture, because a command frame
// is answered later than the hand that sent it; and a refusal that
// carries its reason, because a control a person cannot use owes them
// the sentence saying why.
//
// The words are the caller's: this file holds no prose.

import { Show } from "solid-js";
import type { JSX } from "solid-js";

import { Tip } from "./tip";

// primary is the one thing the screen is for, secondary the ones beside
// it, quiet the ones that must not compete, destructive the one that
// cannot be taken back.
export type Tone = "primary" | "secondary" | "quiet" | "destructive";

// What the control is doing, as one word the stylesheet reads off the
// element. Every posture the button can take is a value here, so a
// reader asks the DOM what state it is in rather than deducing it from
// which classes happen to be on it.
type State = "idle" | "loading" | "stopped";

// The resting paint of each tone, and the only hover it answers.
//
// The hover is written into the idle state rather than beside it: a
// control that is waiting for an acknowledgement, or one a person may
// not use, keeps its muted paint under the pointer instead of lighting
// up as though the press would land.
const PAINT: Record<Tone, string> = {
  primary: "bg-accent text-g0 data-[state=idle]:hover:bg-accent-hover",
  secondary: "bg-g2 text-text data-[state=idle]:hover:bg-g3",
  quiet: "text-text-quiet data-[state=idle]:hover:bg-g2",
  destructive:
    "bg-g2 text-alert data-[state=idle]:hover:bg-alert data-[state=idle]:hover:text-g0",
};

// What every tone looks like once it is no longer idle. One rule for
// both remaining states, because loading and refused are the same
// answer to the hand: not now.
const MUTED = "not-data-[state=idle]:bg-g2 not-data-[state=idle]:text-text-disabled";

// The shape, and the four properties that travel when it changes.
// Background colour is among them so a hover arrives rather than
// switching, which is what tells a hand the control heard it.
const SHAPE =
  "inline-flex items-center gap-snug rounded-control px-base py-tight text-label " +
  "transition-[background-color,color,opacity,transform] duration-100 ease-standard " +
  "active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100";

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

export function Button(props: ButtonProps) {
  // The one reading of what the control is doing. The attribute, the two
  // ARIA properties and the click guard are four readers of this line,
  // and nothing else decides the question.
  const state = (): State => {
    if (props.why !== undefined) return "stopped";
    return props.loading === true ? "loading" : "idle";
  };
  // One definition of the control, drawn bare or inside its reason.
  const control = (hint?: string): JSX.Element => (
    <button
      type={props.type ?? "button"}
      data-state={state()}
      class={`${SHAPE} ${PAINT[props.tone ?? "secondary"]} ${MUTED}`}
      aria-disabled={state() !== "idle"}
      aria-busy={state() === "loading"}
      aria-describedby={hint}
      onClick={() => {
        if (state() !== "idle") return;
        props.onPress?.();
      }}
    >
      <Show when={state() === "loading"}>
        <span class="inline-block size-dot animate-pulse rounded-pill bg-current" />
      </Show>
      {props.label}
    </button>
  );
  return <Show when={props.why} fallback={control()}>{(why) => <Tip text={why()}>{(hint) => control(hint)}</Tip>}</Show>;
}
