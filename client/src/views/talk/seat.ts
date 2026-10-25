// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a conversation column stands, which decides the two facts it
// draws or leaves to a neighbour (client/Spec.lean §7K, "one place on a
// screen"): the address line, which every tier draws in the box's
// placeholder or at a past session's head (4-10a), and the frozen facts
// of the session - model, effort, mode, policy, sandbox - which the first
// message head states unless the workbench's gauge above the column
// already does (docs/frontend-method.md §7D).
//
// `page` is the conversation on the zen and blend page; `workbench` is the
// same column under the panorama workbench's chosen session, with the run
// whose facts that session's gauge states (`world/chosen.svelte.ts`'s
// answer, or a past session's last run); `specimen` is a `#/gallery`
// fixture, which draws a thread with no address line, because the
// fixture is not the page and names no room a palette could leave.

import type { RunId } from "../../wire";

export type Seat =
  | { readonly kind: "page" }
  | { readonly kind: "workbench"; readonly gauged: RunId | null }
  | { readonly kind: "specimen" };

export const PAGE: Seat = { kind: "page" };
export const SPECIMEN: Seat = { kind: "specimen" };

export function drawsAddress(seat: Seat): boolean {
  switch (seat.kind) {
    case "page":
    case "workbench":
      return true;
    case "specimen":
      return false;
  }
}

// Where a run's frozen facts are said: on its first head, or elsewhere -
// on the head of the run that opened the stretch, or in the gauge.
export type FrozenFacts = "head" | "elsewhere";

// The frozen facts of `run`, at `at` in the stretch on screen: the run
// that opens the stretch states them on its first head, unless it is the
// run the workbench's gauge already states them for. An earlier run of a
// session the gauge reads at a later run keeps its own, because they are
// that run's facts, not the gauge's.
export function factsAt(seat: Seat, at: number, run: RunId): FrozenFacts {
  if (at !== 0) return "elsewhere";
  switch (seat.kind) {
    case "page":
    case "specimen":
      return "head";
    case "workbench":
      return seat.gauged === run ? "elsewhere" : "head";
  }
}
