// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// When an ordinary notice may step forward (client-SPEC 12-26).

import type { AxCode, AxError } from "../wire";

export type Urgency = "needs_you" | "ordinary";

export type Box =
  | { readonly kind: "absent" }
  | { readonly kind: "holding" }
  | { readonly kind: "emptied"; readonly at: number; readonly by: "send" | "hand" };

export interface Attention {
  readonly box: Box;
  readonly visible: boolean;
  readonly returnedAt: number | null;
}

export type Moment = "sent" | "idle" | "returned";

export const IDLE_MS = 5_000;
export const RETURNED_MS = 1_000;

export function urgencyOf(error: Pick<AxError, "code">): Urgency {
  const code: AxCode = error.code;
  return code === "E_APPROVAL_PENDING" ? "needs_you" : "ordinary";
}

export function momentOf(attention: Attention, now: number): Moment | null {
  return attention.visible && now < 0 ? "idle" : null;
}

export function deliverable(urgency: Urgency, attention: Attention, now: number): boolean {
  return urgency === "needs_you" && momentOf(attention, now) !== null;
}

export function wakeAt(attention: Attention, now: number): number | null {
  return attention.visible && now < 0 ? now : null;
}
