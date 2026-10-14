// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Branching a session, then going to it: the one door every fork entry
// uses - a mailbox row, a turn of an earlier session, a turn of the
// conversation in main (client/spec/Views/Talk/Branching.lean, client
// D100). There is one of it per page (`ui().branches`), so the corner
// can tell at the moment a refusal lands whether a branch row draws it.
//
// **Main moves once the city has opened the branch.** `send` says only
// that the frame left the page; the city refuses a new session under a
// working run (`E_BUSY`), and a page that had already moved would leave
// the person typing into that run. The wire does not say which command
// a refusal answers, so the answer is read off the belief, as
// `handing.ts` reads a dispatch's: the room's session newer than the
// one held at the press is the branch, and a refusal newer than the one
// held at the press refuses it. When both land in one paint the session
// wins, because the branch then exists.

import { get, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Belief } from "../../core/belief/shape";
import { openSession } from "../../core/commands";
import type { View } from "../../core/route";
import type { Address, AxError, Command, Origin, Seq } from "../../wire";

export type Branching =
  | { readonly kind: "idle" }
  // Sent, and waiting for the city: what the belief held at the press
  // is what an answer must be newer than.
  | {
      readonly kind: "opening";
      readonly room: Address;
      readonly origin: Origin;
      readonly session: Seq | null;
      readonly refusal: AxError | null;
    }
  // Held until the next press, so the row that started it keeps saying
  // why.
  | { readonly kind: "refused"; readonly room: Address; readonly origin: Origin; readonly error: AxError };

export const NOT_BRANCHING: Branching = { kind: "idle" };

export interface Branches {
  readonly held: Readable<Branching>;
  // Sends the branch unless one is already opening; once the belief
  // holds it, main goes to the room and `went` runs.
  readonly open: (room: Address, origin: Origin, went: () => void) => void;
}

// What the door reaches for: the page's belief, its socket and its
// address bar.
export interface BranchDoors {
  readonly belief: Readable<Belief>;
  readonly send: (command: Command) => boolean;
  readonly go: (view: View) => void;
}

// Lives as long as the page: the subscription is the page's own.
export function keepBranches(doors: BranchDoors): Branches {
  const held = writable<Branching>(NOT_BRANCHING);
  let went: () => void = () => undefined;
  doors.belief.subscribe((belief) => {
    const was = get(held);
    const next = settled(was, belief);
    if (next === was) return;
    held.set(next);
    if (was.kind === "opening" && next.kind === "idle") {
      doors.go({ kind: "talk", address: was.room });
      went();
    }
  });
  return {
    held: { subscribe: held.subscribe },
    open: (room, origin, then) => {
      const next = pressed(get(held), get(doors.belief), room, origin);
      if (next === null) return;
      if (!doors.send(openSession(room, "nothing", origin))) {
        held.set(NOT_BRANCHING);
        return;
      }
      went = then;
      held.set(next);
    },
  };
}

// The branch a press starts, or `null` while another one is opening:
// one at a time, because an answer is read by its room and its order,
// and two branches waiting at once could each take the other's.
function pressed(held: Branching, belief: Belief, room: Address, origin: Origin): Branching | null {
  if (held.kind === "opening") return null;
  return { kind: "opening", room, origin, session: belief.sessions[room] ?? null, refusal: belief.refusal };
}

// The branch after the belief moved; the same value when nothing
// answered it.
function settled(held: Branching, belief: Belief): Branching {
  if (held.kind !== "opening") return held;
  if ((belief.sessions[held.room] ?? 0) > (held.session ?? 0)) return NOT_BRANCHING;
  const refusal = belief.refusal;
  if (refusal === null || refusal === held.refusal) return held;
  return { kind: "refused", room: held.room, origin: held.origin, error: refusal };
}

// Whether a branch row draws this refusal, so the corner lets it pass.
export function branchClaims(branching: Branching, refusal: AxError): boolean {
  return branching.kind === "refused" && branching.error === refusal;
}

// What one place that branches shows of the page's branch: its own,
// opening or refused; another one opening, which it waits for; or
// nothing.
export type BranchHere =
  | { readonly kind: "none" }
  | { readonly kind: "opening" }
  | { readonly kind: "refused"; readonly error: AxError }
  | { readonly kind: "elsewhere" };

// For a control that branches from one line, such as a mailbox row.
export function branchFrom(branching: Branching, room: Address, origin: Origin): BranchHere {
  return here(branching, branching.kind !== "idle" && branching.room === room && sameOrigin(branching.origin, origin));
}

// For a place that branches from any line of one room, such as an
// earlier session in main.
export function branchInto(branching: Branching, room: Address): BranchHere {
  return here(branching, branching.kind !== "idle" && branching.room === room);
}

function here(branching: Branching, mine: boolean): BranchHere {
  switch (branching.kind) {
    case "idle":
      return { kind: "none" };
    case "opening":
      return mine ? { kind: "opening" } : { kind: "elsewhere" };
    case "refused":
      return mine ? { kind: "refused", error: branching.error } : { kind: "none" };
  }
}

function sameOrigin(left: Origin, right: Origin): boolean {
  return left.run === right.run && left.at_seq === right.at_seq;
}
