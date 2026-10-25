// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Opening a session, then going to it: the one door every entry that
// opens one uses - a mailbox row, a turn of an earlier session, a turn
// of the conversation in main, the palette's carry rows, `/new` and
// `/compact` (client/spec/Views/Talk/Branching.lean, client D100). There
// is one of it per page (`ui().branches`), so the corner can tell at the
// moment a refusal lands whether a place draws it.
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
import type { Address, AxError, Carry, Command, Origin, Seq } from "../../wire";

// Where the new session begins: the conversation up to one line of an
// earlier session, or a fresh start that carries the room's handoff or
// nothing.
export type Opening = { readonly kind: "branch"; readonly origin: Origin } | { readonly kind: "fresh"; readonly carry: Carry };

export type Branching =
  | { readonly kind: "idle" }
  // Sent, and waiting for the city: what the belief held at the press
  // is what an answer must be newer than.
  | {
      readonly kind: "opening";
      readonly room: Address;
      readonly opening: Opening;
      readonly session: Seq | null;
      readonly refusal: AxError | null;
    }
  // Held until the next press, so the place that started it keeps saying
  // why.
  | { readonly kind: "refused"; readonly room: Address; readonly opening: Opening; readonly error: AxError };

export const NOT_BRANCHING: Branching = { kind: "idle" };

export interface Branches {
  readonly held: Readable<Branching>;
  // Sends the session unless one is already opening, and says whether
  // the frame left the page; once the belief holds the session, main
  // goes to the room and `went` runs.
  readonly open: (room: Address, opening: Opening, went: () => void) => boolean;
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
    open: (room, opening, then) => {
      const next = pressed(get(held), get(doors.belief), room, opening);
      if (next === null) return false;
      const sent =
        opening.kind === "branch" ? openSession(room, "nothing", opening.origin) : openSession(room, opening.carry, null);
      if (!doors.send(sent)) {
        held.set(NOT_BRANCHING);
        return false;
      }
      went = then;
      held.set(next);
      return true;
    },
  };
}

// The opening a press starts, or `null` while another one is opening:
// one at a time, because an answer is read by its room and its order,
// and two sessions waiting at once could each take the other's.
function pressed(held: Branching, belief: Belief, room: Address, opening: Opening): Branching | null {
  if (held.kind === "opening") return null;
  return { kind: "opening", room, opening, session: belief.sessions[room] ?? null, refusal: belief.refusal };
}

// The branch after the belief moved; the same value when nothing
// answered it.
function settled(held: Branching, belief: Belief): Branching {
  if (held.kind !== "opening") return held;
  if ((belief.sessions[held.room] ?? 0) > (held.session ?? 0)) return NOT_BRANCHING;
  const refusal = belief.refusal;
  if (refusal === null || refusal === held.refusal) return held;
  return { kind: "refused", room: held.room, opening: held.opening, error: refusal };
}

// Whether a place draws this refusal, so the corner lets it pass.
export function branchClaims(branching: Branching, refusal: AxError): boolean {
  return branching.kind === "refused" && branching.error === refusal;
}

// What one place that opens sessions shows of the page's opening: its
// own, opening or refused, and whether it is a branch or a fresh start,
// which decides the words; another one opening, which it waits for; or
// nothing.
export type BranchHere =
  | { readonly kind: "none" }
  | { readonly kind: "opening"; readonly of: Opening["kind"] }
  | { readonly kind: "refused"; readonly of: Opening["kind"]; readonly error: AxError }
  | { readonly kind: "elsewhere" };

// For a control that branches from one line, such as a mailbox row.
export function branchFrom(branching: Branching, room: Address, origin: Origin): BranchHere {
  return here(
    branching,
    branching.kind !== "idle" &&
      branching.room === room &&
      branching.opening.kind === "branch" &&
      sameOrigin(branching.opening.origin, origin),
  );
}

// For a place that opens sessions in one room, such as an earlier
// session in main or the conversation in it.
export function branchInto(branching: Branching, room: Address): BranchHere {
  return here(branching, branching.kind !== "idle" && branching.room === room);
}

function here(branching: Branching, mine: boolean): BranchHere {
  switch (branching.kind) {
    case "idle":
      return { kind: "none" };
    case "opening":
      return mine ? { kind: "opening", of: branching.opening.kind } : { kind: "elsewhere" };
    case "refused":
      return mine ? { kind: "refused", of: branching.opening.kind, error: branching.error } : { kind: "none" };
  }
}

function sameOrigin(left: Origin, right: Origin): boolean {
  return left.run === right.run && left.at_seq === right.at_seq;
}
