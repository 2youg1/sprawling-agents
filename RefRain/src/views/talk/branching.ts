// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { get, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Belief } from "../../core/belief/shape";
import { openSession } from "../../core/commands";
import type { View } from "../../core/route";
import type { Address, AxError, Carry, Command, Origin, Seq } from "../../wire";


export type Opening =
  | { readonly kind: "branch"; readonly origin: Origin }
  | { readonly kind: "fresh"; readonly carry: Carry };

export type Branching =
  | { readonly kind: "idle" }
  
  
  | {
      readonly kind: "opening";
      readonly room: Address;
      readonly opening: Opening;
      readonly session: Seq | null;
      readonly refusal: AxError | null;
    }
  
  
  | {
      readonly kind: "refused";
      readonly room: Address;
      readonly opening: Opening;
      readonly error: AxError;
    };

export const NOT_BRANCHING: Branching = { kind: "idle" };

export interface Branches {
  readonly held: Readable<Branching>;
  
  
  readonly open: (room: Address, opening: Opening, went: () => void) => boolean;
}


export interface BranchDoors {
  readonly belief: Readable<Belief>;
  readonly send: (command: Command) => boolean;
  readonly go: (view: View) => void;
}


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
        opening.kind === "branch"
          ? openSession(room, "nothing", opening.origin)
          : openSession(room, opening.carry, null);
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


function pressed(
  held: Branching,
  belief: Belief,
  room: Address,
  opening: Opening,
): Branching | null {
  if (held.kind === "opening") return null;
  return {
    kind: "opening",
    room,
    opening,
    session: belief.sessions[room] ?? null,
    refusal: belief.refusal,
  };
}


function settled(held: Branching, belief: Belief): Branching {
  if (held.kind !== "opening") return held;
  if ((belief.sessions[held.room] ?? 0) > (held.session ?? 0)) return NOT_BRANCHING;
  const refusal = belief.refusal;
  if (refusal === null || refusal === held.refusal) return held;
  return { kind: "refused", room: held.room, opening: held.opening, error: refusal };
}


export function branchClaims(branching: Branching, refusal: AxError): boolean {
  return branching.kind === "refused" && branching.error === refusal;
}


export type BranchHere =
  | { readonly kind: "none" }
  | { readonly kind: "opening"; readonly of: Opening["kind"] }
  | { readonly kind: "refused"; readonly of: Opening["kind"]; readonly error: AxError }
  | { readonly kind: "elsewhere" };


export function branchFrom(branching: Branching, room: Address, origin: Origin): BranchHere {
  return here(
    branching,
    branching.kind !== "idle" &&
      branching.room === room &&
      branching.opening.kind === "branch" &&
      sameOrigin(branching.opening.origin, origin),
  );
}


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
      return mine
        ? { kind: "refused", of: branching.opening.kind, error: branching.error }
        : { kind: "none" };
  }
}

function sameOrigin(left: Origin, right: Origin): boolean {
  return left.run === right.run && left.at_seq === right.at_seq;
}
