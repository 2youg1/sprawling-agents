// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the stack of refusals over the composer decides without the page
// (`refusal.svelte` is the seat, `refusal.look.svelte` draws): where the
// stack stands, and how long a toast has left while a pointer or the
// focus holds it.
//
// **A toast is paused while anything holds it.** The pointer and the
// focus are two holders; the clock stops when the first arrives and
// starts again only when the last one leaves, so a keyboard reader
// whose pointer happens to cross the toast is not raced either.

import type { Snippet } from "svelte";

// Where the stack stands: on the composer's upper edge, as wide as its
// column; centred at the foot of a page with no composer; or, in
// `#/gallery`, in the flow of the fold that shows it.
export type Stand =
  | { readonly kind: "composer"; readonly left: number; readonly width: number; readonly bottom: number }
  | { readonly kind: "foot" }
  | { readonly kind: "specimen" };

export type Holder = "pointer" | "focus";

export interface Clock {
  // How much of its life is left, and when the stretch being counted
  // began.
  readonly left: number;
  readonly since: number;
  readonly holders: readonly Holder[];
}

// The clock once `by` takes hold at `now`. The first holder stops it
// and banks the stretch that ran.
export function held(clock: Clock, by: Holder, now: number): Clock {
  if (clock.holders.includes(by)) return clock;
  const left = clock.holders.length === 0 ? Math.max(0, clock.left - (now - clock.since)) : clock.left;
  return { left, since: now, holders: [...clock.holders, by] };
}

// The clock once `by` lets go at `now`, and whether it runs again: only
// when nothing holds it any more.
export function released(clock: Clock, by: Holder, now: number): { readonly clock: Clock; readonly runs: boolean } {
  if (!clock.holders.includes(by)) return { clock, runs: false };
  const holders = clock.holders.filter((each) => each !== by);
  return { clock: { left: clock.left, since: now, holders }, runs: holders.length === 0 };
}

// What a focus leaving reads: whether the focus went somewhere else
// inside the same toast, which is not letting go.
export type FocusOut = Pick<FocusEvent, "currentTarget" | "relatedTarget">;

export function within(event: FocusOut): boolean {
  const { currentTarget, relatedTarget } = event;
  return currentTarget instanceof Element && relatedTarget instanceof Node && currentTarget.contains(relatedTarget);
}

export interface ToastWire {
  readonly onmouseenter: () => void;
  readonly onmouseleave: () => void;
  readonly onfocusin: () => void;
  readonly onfocusout: (event: FocusOut) => void;
}

export interface ToastLook {
  // A key for `#each`, and what `body` is called with.
  readonly key: number;
  // Leaving: the look plays the departure, and the next arrival sweeps
  // it out of the list.
  readonly gone: boolean;
  readonly wire: ToastWire;
}

export interface RefusalLook {
  readonly stand: Stand;
  readonly toasts: readonly ToastLook[];
  // What one toast says, drawn by the seat: the refusal's notice and
  // its recoveries.
  readonly body: Snippet<[number]>;
}
