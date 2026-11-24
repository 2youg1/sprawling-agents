// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../core/snippet";

export type Stand =
  | {
      readonly kind: "composer";
      readonly left: number;
      readonly width: number;
      readonly bottom: number;
    }
  | { readonly kind: "foot" }
  | { readonly kind: "specimen" };

export type Holder = "pointer" | "focus";

export interface Clock {
  readonly left: number;
  readonly since: number;
  readonly holders: readonly Holder[];
}

export function held(clock: Clock, by: Holder, now: number): Clock {
  if (clock.holders.includes(by)) return clock;
  const left =
    clock.holders.length === 0 ? Math.max(0, clock.left - (now - clock.since)) : clock.left;
  return { left, since: now, holders: [...clock.holders, by] };
}

export function released(
  clock: Clock,
  by: Holder,
  now: number,
): { readonly clock: Clock; readonly runs: boolean } {
  if (!clock.holders.includes(by)) return { clock, runs: false };
  const holders = clock.holders.filter((each) => each !== by);
  return { clock: { left: clock.left, since: now, holders }, runs: holders.length === 0 };
}

export type FocusOut = Pick<FocusEvent, "currentTarget" | "relatedTarget">;

export function within(event: FocusOut): boolean {
  const { currentTarget, relatedTarget } = event;
  return (
    currentTarget instanceof Element &&
    relatedTarget instanceof Node &&
    currentTarget.contains(relatedTarget)
  );
}

export interface ToastWire {
  readonly onmouseenter: () => void;
  readonly onmouseleave: () => void;
  readonly onfocusin: () => void;
  readonly onfocusout: (event: FocusOut) => void;
}

export interface ToastLook {
  readonly key: number;
  readonly gone: boolean;
  readonly wire: ToastWire;
}

export interface RefusalLook {
  readonly stand: Stand;
  readonly toasts: readonly ToastLook[];
  readonly body: Snippet<[number]>;
}
