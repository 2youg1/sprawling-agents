// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A small fact attached to something larger: how many are waiting, or
// where a thing stands. The word is always drawn, and the colour only
// repeats it - a state told by colour alone is a state that is not told
// to everybody.

import { Show } from "solid-js";

// Nothing in particular, something going on, something wrong.
export type Weight = "quiet" | "live" | "alert";

const PAINT: Record<Weight, string> = {
  quiet: "bg-g2 text-text-quiet",
  live: "bg-g2 text-accent",
  alert: "bg-g2 text-alert",
};

const DOT: Record<Weight, string> = {
  quiet: "bg-g5",
  live: "bg-accent",
  alert: "bg-alert",
};

export interface BadgeProps {
  // Already in the person's language, or a number the caller formatted.
  readonly text: string;
  readonly weight?: Weight;
  // A state reads better with a mark beside it; a count does not.
  readonly dot?: boolean;
}

export function Badge(props: BadgeProps) {
  const weight = () => props.weight ?? "quiet";
  return (
    <span
      class={`inline-flex items-center gap-tight rounded-pill px-snug py-tight text-note whitespace-nowrap ${PAINT[weight()]}`}
    >
      <Show when={props.dot === true}>
        <span class={`inline-block size-dot rounded-pill ${DOT[weight()]}`} aria-hidden="true" />
      </Show>
      {props.text}
    </span>
  );
}
