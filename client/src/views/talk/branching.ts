// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Branching a session, then going to it: the one door every fork entry
// uses (client/spec/Views/Talk/Branching.lean, client D100). This is
// what the two entry points do today, gathered in one place: the frame
// leaves the page and main moves in the same tick.

import { readable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Belief } from "../../core/belief/shape";
import { openSession } from "../../core/commands";
import type { View } from "../../core/route";
import type { Address, AxError, Command, Origin, Seq } from "../../wire";

export type Branching =
  | { readonly kind: "idle" }
  | {
      readonly kind: "opening";
      readonly room: Address;
      readonly origin: Origin;
      readonly session: Seq | null;
      readonly refusal: AxError | null;
    }
  | { readonly kind: "refused"; readonly room: Address; readonly origin: Origin; readonly error: AxError };

export const NOT_BRANCHING: Branching = { kind: "idle" };

export interface Branches {
  readonly held: Readable<Branching>;
  readonly open: (room: Address, origin: Origin, went: () => void) => void;
}

export interface BranchDoors {
  readonly belief: Readable<Belief>;
  readonly send: (command: Command) => boolean;
  readonly go: (view: View) => void;
}

export function keepBranches(doors: BranchDoors): Branches {
  return {
    held: readable(NOT_BRANCHING),
    open: (room, origin, went) => {
      if (!doors.send(openSession(room, "nothing", origin))) return;
      doors.go({ kind: "talk", address: room });
      went();
    },
  };
}

export function branchClaims(_branching: Branching, _refusal: AxError): boolean {
  return false;
}
