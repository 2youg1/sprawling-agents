// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// When a notice may step forward on its own, and when it only marks the
// mailbox key (client D26). The one judgement; the toast seat
// and the mailbox key both read it, and `views/mailbox/attention.ts`
// reads the page into the `Attention` it judges.
//
// **Two kinds, by whether the work can move without the person.** What
// cannot - a door waiting on the person's own hand, a city with no
// model or no key, a ledger or a store it can no longer trust - is
// delivered at once, because deferring it only makes the person learn
// later that they are the one holding things up. Everything else is
// ordinary, and waits for a moment.
//
// **The moments are breakpoints in the person's writing** (Horvitz et
// al. 2005; Iqbal & Bailey 2008): a sentence just sent, a box left
// empty for a while, a return to this tab. The box is the one fact this
// page can read about whether the person is in the middle of saying
// something, so every moment is read off it.

import type { AxCode, AxError } from "../wire";

export type Urgency = "needs_you" | "ordinary";

// Every code has a row, so a new code is a decision here rather than a
// silence. `needs_you` is the short list: nothing moves until the
// person acts, and no retry of the city's own resolves it.
const URGENCY: Readonly<Record<AxCode, Urgency>> = {
  // A door asks for the person's own action on the browser they are
  // signed into (kernel §8-27); the run waits on that hand.
  E_APPROVAL_PENDING: "needs_you",
  // Nothing can be dispatched until a model, a key or a provider the
  // city can speak to is in place.
  E_MODEL_UNCHOSEN: "needs_you",
  E_CREDENTIAL_MISSING: "needs_you",
  E_ENDPOINT_DIALECT_UNSUPPORTED: "needs_you",
  // The room is frozen against its session: only `/new` or `/fork`,
  // which the person types, moves it.
  E_CONFIG_INVALID: "needs_you",
  // The city stops spending until the person raises what it may spend.
  E_BUDGET_EXHAUSTED: "needs_you",
  // A standing goal cannot pursue a plan that does not exist.
  E_PLAN_MISSING: "needs_you",
  // The page and the city disagree about the wire; only a reload helps.
  E_WIRE_MISMATCH: "needs_you",
  // The record itself is in doubt: nothing built on it should go on.
  E_CAS_CORRUPT: "needs_you",
  E_STORAGE_FATAL: "needs_you",
  E_LEDGER_HELD: "needs_you",
  E_LOG_VERSION_UNSUPPORTED: "needs_you",
  E_HISTORY_UNPROVEN: "needs_you",
  E_DIGEST_SUSPECT: "needs_you",
  // A refused step inside a run that goes on, or a person's request the
  // city answered: the run or the page carries on without the person.
  E_PATH_NOT_FOUND: "ordinary",
  E_TOOL_UNKNOWN: "ordinary",
  E_TOOL_UNAVAILABLE: "ordinary",
  E_INVALID_ARGS: "ordinary",
  E_OUTSIDE_WRITE_DOMAIN: "ordinary",
  E_VERSION_CONFLICT: "ordinary",
  E_GATE_DENIED: "ordinary",
  E_TIMEOUT: "ordinary",
  E_PROVIDER: "ordinary",
  E_EVIDENCE_MISSING: "ordinary",
  E_LOOP_SUSPECTED: "ordinary",
  E_LOCATOR_INVALID: "ordinary",
  E_SANDBOX_DENIED: "ordinary",
  E_BUSY: "ordinary",
  E_DRAFT_STALE: "ordinary",
  E_GOAL_CONFLICT: "ordinary",
  E_TAINTED_ACTION: "ordinary",
  E_REPAIR_BUSY: "ordinary",
  E_DELEGATION_DEPTH: "ordinary",
  E_APPROVAL_DENIED: "ordinary",
  E_CROSS_BUILDING_DENIED: "ordinary",
  E_WORKTREE_BUSY: "ordinary",
  E_BROWSER_UNAVAILABLE: "ordinary",
  E_SECRET_EGRESS: "ordinary",
  E_DISCARD_IRREVERSIBLE: "ordinary",
  E_BACKPRESSURE_SHED: "ordinary",
  E_TOOL_OUTCOME_UNKNOWN: "ordinary",
};

export function urgencyOf(error: Pick<AxError, "code">): Urgency {
  return URGENCY[error.code];
}

// The composer's box as the moments read it: no box on this page, words
// in it, or empty since a moment and emptied either by a send or by the
// person's own hand (which also covers a box that arrived empty).
export type Box =
  | { readonly kind: "absent" }
  | { readonly kind: "holding" }
  | { readonly kind: "emptied"; readonly at: number; readonly by: "send" | "hand" };

export interface Attention {
  readonly box: Box;
  // Whether the tab is shown at all. A hidden tab has no moment: a toast
  // there would leave before anybody saw it.
  readonly visible: boolean;
  // When the tab was last shown again after being hidden.
  readonly returnedAt: number | null;
}

export type Moment = "sent" | "idle" | "returned";

// How long a box emptied by hand stays empty before the person is taken
// to have stopped writing: clearing a box is often the start of
// rewriting it.
export const IDLE_MS = 5_000;
// How long a return to the tab counts as the return itself: long enough
// for what queued while it was hidden to fold in the first frame after
// it is shown (client/Spec.lean §4-11), short enough that typing resumed
// after it is no longer interrupted.
export const RETURNED_MS = 1_000;

export function momentOf(attention: Attention, now: number): Moment | null {
  if (!attention.visible) return null;
  if (attention.returnedAt !== null && now - attention.returnedAt <= RETURNED_MS) return "returned";
  const box = attention.box;
  switch (box.kind) {
    case "absent":
      return "idle";
    case "holding":
      return null;
    case "emptied":
      if (box.by === "send") return "sent";
      return now - box.at >= IDLE_MS ? "idle" : null;
  }
}

export function deliverable(urgency: Urgency, attention: Attention, now: number): boolean {
  switch (urgency) {
    case "needs_you":
      return true;
    case "ordinary":
      return momentOf(attention, now) !== null;
  }
}

// The instant a moment opens if nothing else happens: a box cleared by
// hand that keeps being empty. Every other moment opens on an event -
// a send, a return, the box arriving or leaving - which the reader of
// `Attention` hears for itself.
export function wakeAt(attention: Attention, now: number): number | null {
  const box = attention.box;
  if (!attention.visible || box.kind !== "emptied" || box.by !== "hand") return null;
  const due = box.at + IDLE_MS;
  return due > now ? due : null;
}
