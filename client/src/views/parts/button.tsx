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

// primary is the one thing the screen is for, secondary the ones beside
// it, quiet the ones that must not compete, destructive the one that
// cannot be taken back.
export type Tone = "primary" | "secondary" | "quiet" | "destructive";

const PAINT: Record<Tone, string> = {
  primary: "bg-accent text-g0 hover:bg-accent-hover",
  secondary: "bg-g2 text-text hover:bg-g3",
  quiet: "text-text-quiet hover:bg-g2",
  destructive: "bg-g2 text-alert hover:bg-alert hover:text-g0",
};

export interface ButtonProps {
  // Already in the person's language: a `say` key resolved by the page.
  readonly label: string;
  readonly tone?: Tone;
  readonly type?: "button" | "submit";
  // The command went out and its acknowledgement has not come back.
  readonly loading?: boolean;
  // Present means the control cannot be used, and says why. It stays
  // focusable and keeps `aria-disabled`, because a `disabled` element is
  // skipped by the keyboard and its reason is never read out.
  readonly why?: string;
  readonly onPress?: () => void;
}

export function Button(props: ButtonProps) {
  const stopped = () => props.why !== undefined || props.loading === true;
  const paint = () => (stopped() ? "bg-g2 text-text-disabled" : PAINT[props.tone ?? "secondary"]);
  return (
    <button
      type={props.type ?? "button"}
      class={`inline-flex items-center gap-snug rounded-control px-base py-tight text-label transition-[opacity,transform] duration-100 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 ${paint()}`}
      aria-disabled={stopped()}
      aria-busy={props.loading === true}
      title={props.why}
      onClick={() => {
        if (stopped()) return;
        props.onPress?.();
      }}
    >
      <Show when={props.loading === true}>
        <span class="inline-block size-dot animate-pulse rounded-pill bg-current" />
      </Show>
      {props.label}
    </button>
  );
}
