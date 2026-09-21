// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a run is doing, and where a message typed right now lands.
//
// **The posture has one derivation.** The stream says it by carrying a
// record, an answer says it by naming a run's last kind, and neither
// route was allowed to invent a phase the other would not: the six
// kinds that state one are listed here, once, and `moves` is the only
// place the client asks whether a kind says anything at all. A page
// that has been told nothing is `unknown`, which is a fact and not a
// guess - the record that settled the phase was never read here, and
// `thinking` answered that question wrongly for every run stopped at
// an approval.

import type { EventKind } from "../wire";

export type Doing =
  // This page knows the run exists and has not been told a phase: the
  // records it folded state none, or an answer named a last kind whose
  // record it never saw.
  | { readonly kind: "unknown" }
  | { readonly kind: "thinking" }
  // `tool` is null when the phase is known and the call is not: an
  // answer names a run's last kind, and the tool's name is in the
  // record's payload, which an answer does not carry.
  | { readonly kind: "calling"; readonly tool: string | null; readonly subject: string | null }
  | { readonly kind: "waiting" }
  | { readonly kind: "frozen"; readonly completion: string | null };

// Where a message typed right now will land.
//
// A steer is consumed at a phase boundary, so "it was sent" and "it was
// heard" are not one moment: while a tool call is out, the run is inside
// a system call and the words wait for it to come back. Spelling all
// three the same way tells a person they are in a conversation when they
// are in a queue, which is the one thing a streaming page must not say.
export type Sending = "dispatch" | "steer" | "queued";

// No run, or a frozen one, means the next message opens work rather than
// interrupting it.
export function sendingInto(doing: Doing | undefined): Sending {
  if (doing === undefined) return "dispatch";
  switch (doing.kind) {
    case "frozen":
      return "dispatch";
    case "thinking":
      return "steer";
    // Blocked: inside a tool call, or stopped at an approval nobody has
    // answered. Neither reaches a safe point until it is over, and a
    // phase this page has not been told is not a promise that the words
    // land at a boundary either - so all three go as a queued steer,
    // the spelling that promises a person least.
    case "unknown":
    case "calling":
    case "waiting":
      return "queued";
  }
}

// The six kinds whose record states what the run is doing. A list, so
// that the type below is read off it: a kind named here is a member of
// `Moving`, and the switch in `PHASES` cannot be left part-done without
// the build refusing.
const MOVING = [
  "run_started",
  "model_called",
  "tool_result",
  "tool_called",
  "approval_requested",
  "run_frozen",
] as const;

type Moving = (typeof MOVING)[number];

// Whether a kind states a phase. The predicate is what lets a switch
// over the whole vocabulary hand the kind to `PHASES` below, and it is
// read by `belief.ts` on both routes: the fold has a record, and
// `adopted` has only a `RunSummary::last_kind`.
export function moves(kind: EventKind): kind is Moving {
  return MOVING.some((each) => each === kind);
}

// What each of those kinds says on the reading that has only the kind.
// The fold refines the two whose payload names the call or the ending;
// a page holding an answer has neither payload.
export const PHASES: Record<Moving, Doing> = {
  run_started: { kind: "thinking" },
  model_called: { kind: "thinking" },
  tool_result: { kind: "thinking" },
  approval_requested: { kind: "waiting" },
  tool_called: { kind: "calling", tool: null, subject: null },
  run_frozen: { kind: "frozen", completion: null },
};
